use actix_web::{web, HttpResponse};
use serde_json::json;

use crate::auth::AdminAuthMiddleware;
use crate::db::Db;

/// Configure admin-only utility routes.
/// Pass prefix "/api" for /api/admin, "" for /admin (Trunk proxy).
pub fn configure_routes(
    cfg: &mut web::ServiceConfig,
    db: Db,
    redis_client: std::sync::Arc<redis::Client>,
    prefix: &str,
) {
    let scope_path = if prefix.is_empty() {
        "/admin".to_string()
    } else {
        format!("{}/admin", prefix)
    };

    cfg.service(
        web::scope(&scope_path)
            .wrap(AdminAuthMiddleware {
                redis: redis_client.clone(),
                db: std::sync::Arc::new(db.clone()),
            })
            .route(
                "/cache/analytics/clear",
                web::post().to(|redis: web::Data<redis::Client>| async move {
                    let cache =
                        crate::analytics::cache::RedisAnalyticsCache::new(redis.get_ref().clone());
                    // Clears all `analytics:*` keys.
                    cache.clear().await;
                    HttpResponse::Ok().json(json!({ "ok": true }))
                }),
            ),
    );
}
