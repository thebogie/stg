use actix_web::{web, HttpMessage, HttpResponse};
use serde::Deserialize;
use serde_json::json;

use crate::auth::AdminAuthMiddleware;
use crate::player::repository::{PlayerRepository, PlayerRepositoryImpl};
use crate::sell::playwright_queue;

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
    let players = repo.search_players(q, limit).await;
    let out: Vec<shared::dto::player::PlayerDto> =
        players.iter().map(shared::dto::player::PlayerDto::from).collect();
    HttpResponse::Ok().json(json!({ "users": out }))
}

async fn set_admin_handler(
    path: web::Path<String>,
    body: web::Json<SetAdminBody>,
    repo: web::Data<PlayerRepositoryImpl>,
) -> HttpResponse {
    let player_id = path.into_inner();
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
            .route("/users/{player_id:.*}/admin", web::post().to(set_admin_handler))
            .route("/playwright/smoke", web::post().to(enqueue_smoke_handler)),
    );
}
