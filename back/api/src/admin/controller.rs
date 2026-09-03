use actix_web::{web, HttpMessage, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use shared::dto::player::{
    AdminCreatePlayerRequest, AdminResetPasswordRequest, AdminUpdatePlayerRequest, PlayerDto,
};
use validator::Validate;

use crate::auth::AdminAuthMiddleware;
use crate::player::error::PlayerError;
use crate::player::repository::{PlayerRepository, PlayerRepositoryImpl};
use crate::player::usecase::{PlayerUseCase, PlayerUseCaseImpl};
use crate::sell::playwright_queue;

fn normalize_player_id(player_id: &str) -> String {
    if player_id.contains('/') {
        player_id.to_string()
    } else {
        format!("player/{}", player_id)
    }
}

fn player_error_response(err: PlayerError) -> HttpResponse {
    match err {
        PlayerError::NotFound => HttpResponse::NotFound().json(json!({ "error": err.to_string() })),
        PlayerError::AlreadyExists | PlayerError::ValidationError(_) => {
            HttpResponse::BadRequest().json(json!({ "error": err.to_string() }))
        }
        PlayerError::AccountDisabled => {
            HttpResponse::Forbidden().json(json!({ "error": err.to_string() }))
        }
        PlayerError::InvalidPassword => {
            HttpResponse::Unauthorized().json(json!({ "error": err.to_string() }))
        }
        PlayerError::DatabaseError(msg) | PlayerError::SessionError(msg) => {
            HttpResponse::InternalServerError().json(json!({ "error": msg }))
        }
    }
}

#[derive(Deserialize)]
struct SetAdminBody {
    is_admin: bool,
}

async fn search_users_handler(
    query: web::Query<std::collections::HashMap<String, String>>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> HttpResponse {
    let q = query.get("q").map(String::as_str).unwrap_or("");
    let limit = query
        .get("limit")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(20);
    let players = repo.search_players(q, limit, true).await;
    let out: Vec<shared::dto::player::PlayerDto> =
        players.iter().map(shared::dto::player::PlayerDto::from).collect();
    HttpResponse::Ok().json(json!({ "users": out }))
}

async fn create_user_handler(
    body: web::Json<AdminCreatePlayerRequest>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> HttpResponse {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(json!({ "error": e.to_string() }));
    }
    let usecase = PlayerUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    match usecase.admin_create_player(body.into_inner()).await {
        Ok(player) => {
            log::info!("Admin created player {}", player.id);
            HttpResponse::Created().json(PlayerDto::from(&player))
        }
        Err(e) => player_error_response(e),
    }
}

async fn delete_user_handler(
    path: web::Path<String>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> HttpResponse {
    let player_id = normalize_player_id(&path.into_inner());
    let usecase = PlayerUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    match usecase.admin_delete_player(&player_id).await {
        Ok(()) => {
            log::info!("Admin deleted player {}", player_id);
            HttpResponse::Ok().json(json!({ "ok": true, "message": "Player deleted" }))
        }
        Err(e) => player_error_response(e),
    }
}

async fn deactivate_user_handler(
    path: web::Path<String>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> HttpResponse {
    let player_id = normalize_player_id(&path.into_inner());
    let usecase = PlayerUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    match usecase.admin_set_active(&player_id, false).await {
        Ok(player) => {
            log::info!("Admin deactivated player {}", player_id);
            HttpResponse::Ok().json(PlayerDto::from(&player))
        }
        Err(e) => player_error_response(e),
    }
}

async fn reactivate_user_handler(
    path: web::Path<String>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> HttpResponse {
    let player_id = normalize_player_id(&path.into_inner());
    let usecase = PlayerUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    match usecase.admin_set_active(&player_id, true).await {
        Ok(player) => {
            log::info!("Admin reactivated player {}", player_id);
            HttpResponse::Ok().json(PlayerDto::from(&player))
        }
        Err(e) => player_error_response(e),
    }
}

async fn get_user_handler(
    path: web::Path<String>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> HttpResponse {
    let player_id = normalize_player_id(&path.into_inner());
    match repo.find_by_id(&player_id).await {
        Some(player) => HttpResponse::Ok().json(PlayerDto::from(&player)),
        None => HttpResponse::NotFound().json(json!({ "error": "Player not found" })),
    }
}

async fn update_user_handler(
    path: web::Path<String>,
    body: web::Json<AdminUpdatePlayerRequest>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> HttpResponse {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(json!({ "error": e.to_string() }));
    }
    let player_id = normalize_player_id(&path.into_inner());
    let usecase = PlayerUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    match usecase
        .admin_update_player(&player_id, body.into_inner())
        .await
    {
        Ok(player) => {
            log::info!("Admin updated player {}", player_id);
            HttpResponse::Ok().json(PlayerDto::from(&player))
        }
        Err(e) => player_error_response(e),
    }
}

async fn reset_password_handler(
    path: web::Path<String>,
    body: web::Json<AdminResetPasswordRequest>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> HttpResponse {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(json!({ "error": e.to_string() }));
    }
    let player_id = normalize_player_id(&path.into_inner());
    let usecase = PlayerUseCaseImpl {
        repo: repo.get_ref().clone(),
    };
    let request = body.into_inner();
    match usecase
        .admin_reset_password(&player_id, &request.new_password)
        .await
    {
        Ok(player) => {
            log::info!("Admin reset password for player {}", player_id);
            HttpResponse::Ok().json(json!({
                "message": "Password reset successfully",
                "player": PlayerDto::from(&player),
            }))
        }
        Err(e) => player_error_response(e),
    }
}

async fn set_admin_handler(
    path: web::Path<String>,
    body: web::Json<SetAdminBody>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> HttpResponse {
    let player_id = normalize_player_id(&path.into_inner());
    match repo.set_admin_status(&player_id, body.is_admin).await {
        Ok(player) => HttpResponse::Ok().json(shared::dto::player::PlayerDto::from(&player)),
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}

async fn enqueue_smoke_handler(
    redis: web::Data<redis::Client>,
    req: actix_web::HttpRequest,
) -> HttpResponse {
    let player_id = req
        .extensions()
        .get::<String>()
        .cloned()
        .unwrap_or_else(|| "admin".to_string());
    let base_url = std::env::var("STG_BASE_URL")
        .or_else(|_| std::env::var("PUBLIC_BASE_URL"))
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    match playwright_queue::enqueue_smoke_job(redis.get_ref(), &player_id, &base_url).await {
        Ok(job_id) => HttpResponse::Ok().json(json!({ "ok": true, "job_id": job_id })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e })),
    }
}

/// Configure admin-only utility routes.
/// Pass prefix "/api" for /api/admin, "" for /admin (Trunk proxy).
pub fn configure_routes(
    cfg: &mut web::ServiceConfig,
    redis_client: std::sync::Arc<redis::Client>,
    prefix: &str,
    player_repo: std::sync::Arc<PlayerRepositoryImpl>,
) {
    let scope_path = if prefix.is_empty() {
        "/admin".to_string()
    } else {
        format!("{}/admin", prefix)
    };

    let repo_data = web::Data::from(player_repo.clone());
    let redis_data = web::Data::new((*redis_client).clone());

    cfg.service(
        web::scope(&scope_path)
            .app_data(repo_data)
            .app_data(redis_data)
            .wrap(AdminAuthMiddleware {
                redis: redis_client.clone(),
                player_repo,
            })
            .route(
                "/cache/analytics/clear",
                web::post().to(|redis: web::Data<redis::Client>| async move {
                    let cache =
                        crate::analytics::cache::RedisAnalyticsCache::new(redis.get_ref().clone());
                    cache.clear().await;
                    HttpResponse::Ok().json(json!({ "ok": true }))
                }),
            )
            .route("/users/search", web::get().to(search_users_handler))
            .route("/users", web::post().to(create_user_handler))
            .route(
                "/users/{player_id:.*}/deactivate",
                web::post().to(deactivate_user_handler),
            )
            .route(
                "/users/{player_id:.*}/reactivate",
                web::post().to(reactivate_user_handler),
            )
            .route(
                "/users/{player_id:.*}/password",
                web::post().to(reset_password_handler),
            )
            .route("/users/{player_id:.*}/admin", web::post().to(set_admin_handler))
            .route("/users/{player_id:.*}", web::get().to(get_user_handler))
            .route("/users/{player_id:.*}", web::put().to(update_user_handler))
            .route("/users/{player_id:.*}", web::delete().to(delete_user_handler))
            .route("/playwright/smoke", web::post().to(enqueue_smoke_handler)),
    );
}
