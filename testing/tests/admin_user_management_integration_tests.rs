//! Integration tests: admin player edit and password reset.
//!
//! Requires SurrealDB + Redis: `./deploy/stack.sh start`

use actix_web::{test, web, App};
use anyhow::Result;
use serde_json::json;
use shared::dto::player::{LoginResponse, PlayerDto};
use testing::app_setup;
use testing::create_authenticated_user;
use testing::TestEnvironment;

struct AdminEmailsGuard {
    previous: Option<String>,
}

impl AdminEmailsGuard {
    fn set_to(email: &str) -> Self {
        let previous = std::env::var("ADMIN_EMAILS").ok();
        std::env::set_var("ADMIN_EMAILS", email);
        Self { previous }
    }
}

impl Drop for AdminEmailsGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(v) => std::env::set_var("ADMIN_EMAILS", v),
            None => std::env::remove_var("ADMIN_EMAILS"),
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn admin_can_update_player_and_reset_password() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let admin_email = format!("admin_user_mgmt_{}@example.com", ts);
    let target_email = format!("target_user_mgmt_{}@example.com", ts);
    let _admin_guard = AdminEmailsGuard::set_to(&admin_email);

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.db.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .configure(|cfg| {
                backend::admin::controller::configure_routes(
                    cfg,
                    app_data.redis_arc.clone(),
                    "/api",
                    app_data.player_repo_arc.clone(),
                );
            }),
    )
    .await;

    let admin_session = create_authenticated_user!(app, admin_email.as_str(), "adminmgmt");
    let target_handle = format!("targetmgmt_{}", ts % 100_000);
    let _target_session =
        create_authenticated_user!(app, target_email.as_str(), target_handle.as_str());

    let target_login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": target_email,
            "password": "password123"
        }))
        .to_request();
    let target_login_resp = test::call_service(&app, target_login_req).await;
    assert!(
        target_login_resp.status().is_success(),
        "target login after register"
    );
    let target_login: LoginResponse = test::read_body_json(target_login_resp).await;

    let search_req = test::TestRequest::get()
        .uri(&format!(
            "/api/admin/users/search?q={}&limit=5",
            urlencoding::encode(&target_handle)
        ))
        .insert_header(("Authorization", format!("Bearer {}", admin_session)))
        .to_request();
    let search_resp = test::call_service(&app, search_req).await;
    assert!(search_resp.status().is_success(), "admin search");
    let search_body: serde_json::Value = test::read_body_json(search_resp).await;
    let users: Vec<PlayerDto> = serde_json::from_value(search_body["users"].clone()).unwrap();
    assert!(
        users.iter().any(|u| u.email == target_email),
        "admin search by handle should find target: {:?}",
        search_body
    );

    let new_handle = format!("targetmgmt_upd_{}", ts % 100_000);
    let new_email = format!("target_updated_{}@example.com", ts);
    let encoded_id = urlencoding::encode(&target_login.player.id);

    let update_req = test::TestRequest::put()
        .uri(&format!("/api/admin/users/{}", encoded_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_session)))
        .set_json(&json!({
            "firstname": "Updated Name",
            "handle": new_handle,
            "email": new_email,
            "is_admin": false
        }))
        .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    let update_status = update_resp.status();
    let update_body = test::read_body(update_resp).await;
    assert!(
        update_status.is_success(),
        "admin update player: {}",
        String::from_utf8_lossy(&update_body)
    );
    let updated: PlayerDto = serde_json::from_slice(&update_body)?;
    assert_eq!(updated.firstname, "Updated Name");
    assert_eq!(updated.handle, new_handle);
    assert_eq!(updated.email, new_email);

    let get_req = test::TestRequest::get()
        .uri(&format!("/api/admin/users/{}", encoded_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_session)))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert!(get_resp.status().is_success());
    let fetched: PlayerDto = test::read_body_json(get_resp).await;
    assert_eq!(fetched.email, new_email);

    let reset_req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/password", encoded_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_session)))
        .set_json(&json!({ "new_password": "newpass123" }))
        .to_request();
    let reset_resp = test::call_service(&app, reset_req).await;
    let reset_status = reset_resp.status();
    let reset_body = test::read_body(reset_resp).await;
    assert!(
        reset_status.is_success(),
        "admin password reset: {}",
        String::from_utf8_lossy(&reset_body)
    );

    let old_login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": new_email,
            "password": "password123"
        }))
        .to_request();
    let old_login_resp = test::call_service(&app, old_login_req).await;
    assert!(!old_login_resp.status().is_success(), "old password rejected");

    let new_login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": new_email,
            "password": "newpass123"
        }))
        .to_request();
    let new_login_resp = test::call_service(&app, new_login_req).await;
    assert!(
        new_login_resp.status().is_success(),
        "login with new password"
    );
    let login_body: LoginResponse = test::read_body_json(new_login_resp).await;
    assert_eq!(login_body.player.email, new_email);

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn non_admin_cannot_update_players() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let user_email = format!("nonadmin_user_mgmt_{}@example.com", ts);
    let target_email = format!("nonadmin_target_{}@example.com", ts);

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.db.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .configure(|cfg| {
                backend::admin::controller::configure_routes(
                    cfg,
                    app_data.redis_arc.clone(),
                    "/api",
                    app_data.player_repo_arc.clone(),
                );
            }),
    )
    .await;

    let user_session = create_authenticated_user!(app, user_email.as_str(), "nonadminmgmt");
    let _target_session =
        create_authenticated_user!(app, target_email.as_str(), "nonadmintarget");

    let search_req = test::TestRequest::get()
        .uri(&format!(
            "/api/admin/users/search?q={}&limit=5",
            urlencoding::encode(&target_email)
        ))
        .insert_header(("Authorization", format!("Bearer {}", user_session)))
        .to_request();
    let search_resp = test::call_service(&app, search_req).await;
    assert_eq!(
        search_resp.status(),
        actix_web::http::StatusCode::UNAUTHORIZED,
        "non-admin blocked from admin search"
    );

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn admin_can_create_and_delete_player() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let admin_email = format!("admin_create_del_{}@example.com", ts);
    let new_email = format!("admin_created_{}@example.com", ts);
    let new_handle = format!("admcreated_{}", ts % 100_000);
    let _admin_guard = AdminEmailsGuard::set_to(&admin_email);

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.db.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .configure(|cfg| {
                backend::admin::controller::configure_routes(
                    cfg,
                    app_data.redis_arc.clone(),
                    "/api",
                    app_data.player_repo_arc.clone(),
                );
            }),
    )
    .await;

    let admin_session = create_authenticated_user!(app, admin_email.as_str(), "admincreated");

    let create_req = test::TestRequest::post()
        .uri("/api/admin/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_session)))
        .set_json(&json!({
            "firstname": "Created",
            "handle": new_handle,
            "email": new_email,
            "password": "password123",
            "is_admin": false
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(
        create_resp.status(),
        actix_web::http::StatusCode::CREATED,
        "admin create player: {}",
        String::from_utf8_lossy(&test::read_body(create_resp).await)
    );

    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": new_email,
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    assert!(login_resp.status().is_success(), "created player can login");
    let login_body: LoginResponse = test::read_body_json(login_resp).await;
    let player_id = login_body.player.id;
    let encoded_id = urlencoding::encode(&player_id);

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/admin/users/{}", encoded_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_session)))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert!(
        delete_resp.status().is_success(),
        "admin delete player: {}",
        String::from_utf8_lossy(&test::read_body(delete_resp).await)
    );

    let login_after = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": new_email,
            "password": "password123"
        }))
        .to_request();
    let login_after_resp = test::call_service(&app, login_after).await;
    assert!(
        !login_after_resp.status().is_success(),
        "deleted player cannot login"
    );

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn admin_deactivate_blocks_login() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let admin_email = format!("admin_deact_{}@example.com", ts);
    let target_email = format!("target_deact_{}@example.com", ts);
    let _admin_guard = AdminEmailsGuard::set_to(&admin_email);

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.db.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .configure(|cfg| {
                backend::admin::controller::configure_routes(
                    cfg,
                    app_data.redis_arc.clone(),
                    "/api",
                    app_data.player_repo_arc.clone(),
                );
            }),
    )
    .await;

    let admin_session = create_authenticated_user!(app, admin_email.as_str(), "admindeact");
    let target_handle = format!("targetdeact_{}", ts % 100_000);
    let _target_session =
        create_authenticated_user!(app, target_email.as_str(), target_handle.as_str());

    let target_login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": target_email,
            "password": "password123"
        }))
        .to_request();
    let target_login_resp = test::call_service(&app, target_login_req).await;
    assert!(target_login_resp.status().is_success(), "target login");
    let target_login: LoginResponse = test::read_body_json(target_login_resp).await;
    let encoded_id = urlencoding::encode(&target_login.player.id);

    let deactivate_req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/deactivate", encoded_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_session)))
        .to_request();
    let deactivate_resp = test::call_service(&app, deactivate_req).await;
    assert!(deactivate_resp.status().is_success(), "deactivate");

    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": target_email,
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    assert_eq!(
        login_resp.status(),
        actix_web::http::StatusCode::FORBIDDEN,
        "deactivated player cannot login"
    );

    let reactivate_req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/reactivate", encoded_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_session)))
        .to_request();
    let reactivate_resp = test::call_service(&app, reactivate_req).await;
    assert!(reactivate_resp.status().is_success(), "reactivate");

    let login_ok = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": target_email,
            "password": "password123"
        }))
        .to_request();
    let login_ok_resp = test::call_service(&app, login_ok).await;
    assert!(login_ok_resp.status().is_success(), "reactivated player can login");

    Ok(())
}
