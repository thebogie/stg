use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use serde::Deserialize;
use arangors::client::ClientExt;
use shared::dto::ratings::{RatingScope};

use super::usecase::RatingsUsecase;
use super::repository::RatingsRepository;
use super::scheduler::RatingsScheduler;

#[derive(Clone)]
pub struct RatingsController<C: ClientExt + Send + Sync + 'static> {
    usecase: RatingsUsecase<C>,
    scheduler: web::Data<RatingsScheduler<C>>,
}

impl<C: ClientExt + Send + Sync + 'static> RatingsController<C> {
    pub fn new(usecase: RatingsUsecase<C>, scheduler: RatingsScheduler<C>) -> Self { 
        Self { usecase, scheduler: web::Data::new(scheduler) } 
    }

    #[cfg(test)]
    fn normalize_id(collection: &str, key_or_id: &str) -> String { if key_or_id.contains('/') { key_or_id.to_string() } else { format!("{}/{}", collection, key_or_id) } }

    pub fn configure_routes(cfg: &mut web::ServiceConfig, db: arangors::Database<C>, scheduler: RatingsScheduler<C>, redis: redis::Client) {
        let repo = RatingsRepository::new(db.clone());
        let controller = web::Data::new(RatingsController { 
            usecase: RatingsUsecase::new(repo),
            scheduler: web::Data::new(scheduler)
        });

        cfg.service(
            web::scope("/api/ratings")
                .app_data(controller.clone())
                .route("/recompute", web::post().to(|req: HttpRequest, query: web::Query<RecomputeQuery>, ctrl: web::Data<RatingsController<C>>| async move {
                    ctrl.recompute(req, query.into_inner()).await
                }))
                .route("/leaderboard", web::get().to(|_req: HttpRequest, query: web::Query<LeaderboardQuery>, ctrl: web::Data<RatingsController<C>>| async move {
                    let scope = match query.scope.as_deref() { Some("global") | None => RatingScope::Global, Some(s) if s.starts_with("game/") => RatingScope::Game(s.to_string()), _ => RatingScope::Global };
                    let min_games = query.min_games.unwrap_or(10);
                    let limit = query.limit.unwrap_or(50);
                    match ctrl.usecase.get_leaderboard(scope, min_games, limit).await {
                        Ok(rows) => Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(rows)),
                        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    }
                }))
                .route("/leaderboard/simple", web::get().to(|_req: HttpRequest, query: web::Query<LeaderboardQuery>, ctrl: web::Data<RatingsController<C>>| async move {
                    let scope = match query.scope.as_deref() { Some("global") | None => RatingScope::Global, Some(s) if s.starts_with("game/") => RatingScope::Game(s.to_string()), _ => RatingScope::Global };
                    let min_games = query.min_games.unwrap_or(10);
                    let limit = query.limit.unwrap_or(50);
                    match ctrl.usecase.get_simple_leaderboard(scope, min_games, limit).await {
                        Ok(rows) => Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(rows)),
                        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    }
                }))
                .route("/leaderboard/enhanced", web::get().to(|_req: HttpRequest, query: web::Query<LeaderboardQuery>, ctrl: web::Data<RatingsController<C>>| async move {
                    let scope = match query.scope.as_deref() { Some("global") | None => RatingScope::Global, Some(s) if s.starts_with("game/") => RatingScope::Game(s.to_string()), _ => RatingScope::Global };
                    let min_games = query.min_games.unwrap_or(10);
                    let limit = query.limit.unwrap_or(50);
                    match ctrl.usecase.get_leaderboard_with_contest_data(scope, min_games, limit).await {
                        Ok(rows) => Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(rows)),
                        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    }
                }))
                .route("/debug/player-ids", web::get().to(|_req: HttpRequest, ctrl: web::Data<RatingsController<C>>| async move {
                    match ctrl.usecase.debug_player_ids().await {
                        Ok(debug_info) => Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(debug_info)),
                        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    }
                }))
                .route("/debug/player-period", web::get().to(|_req: HttpRequest, query: web::Query<DebugPlayerPeriodQuery>, ctrl: web::Data<RatingsController<C>>| async move {
                    match ctrl.usecase.debug_player_period_activity(&query.email, &query.period).await {
                        Ok(info) => Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(info)),
                        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    }
                }))
                .route("/debug/collections", web::get().to(|_req: HttpRequest, ctrl: web::Data<RatingsController<C>>| async move {
                    match ctrl.usecase.debug_collections().await {
                        Ok(debug_info) => Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(debug_info)),
                        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    }
                }))
                .route("/debug/resulted-in-vs-players", web::get().to(|_req: HttpRequest, ctrl: web::Data<RatingsController<C>>| async move {
                    match ctrl.usecase.debug_resulted_in_vs_players().await {
                        Ok(debug_info) => Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(debug_info)),
                        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    }
                }))
                .route("/current", web::get().to(|req: HttpRequest, ctrl: web::Data<RatingsController<C>>| async move {
                    // Get current player's email from session
                    let email = match req.extensions().get::<String>() {
                        Some(email) => email.clone(),
                        None => return Ok(HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not authenticated"})))
                    };
                    
                    // Get player ID from email
                    let player_id = match ctrl.usecase.get_player_id_by_email(&email).await {
                        Ok(Some(pid)) => pid,
                        Ok(None) => return Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "Player not found"}))),
                        Err(e) => return Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    };
                    
                    // Get current player's ratings
                    match ctrl.usecase.get_player_ratings(&player_id).await {
                        Ok(rows) => Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(rows)),
                        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    }
                }).wrap(crate::auth::AuthMiddleware { redis: std::sync::Arc::new(redis.clone()) }))
                .route("/history", web::get().to(|req: HttpRequest, query: web::Query<HistoryQuery>, ctrl: web::Data<RatingsController<C>>| async move {
                    // Auth: require session to read email
                    let email = match req.extensions().get::<String>() {
                        Some(email) => email.clone(),
                        None => return Ok(HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not authenticated"})))
                    };

                    // Resolve player_id
                    let player_id = match ctrl.usecase.get_player_id_by_email(&email).await {
                        Ok(Some(pid)) => pid,
                        Ok(None) => return Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "Player not found"}))),
                        Err(e) => return Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    };

                    // Parse scope
                    let scope = match query.scope.as_deref() {
                        Some("global") | None => shared::dto::ratings::RatingScope::Global,
                        Some(s) if s.starts_with("game/") => shared::dto::ratings::RatingScope::Game(s.to_string()),
                        Some(_) => shared::dto::ratings::RatingScope::Global,
                    };

                    // Load history
                    match ctrl.usecase.get_player_rating_history(&player_id, scope).await {
                        Ok(rows) => Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(rows)),
                        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    }
                }).wrap(crate::auth::AuthMiddleware { redis: std::sync::Arc::new(redis.clone()) }))
                .route("/player/{player_id}", web::get().to(|path: web::Path<String>, _req: HttpRequest, ctrl: web::Data<RatingsController<C>>| async move {
                    fn normalize_id(collection: &str, key_or_id: &str) -> String {
                        if key_or_id.contains('/') { key_or_id.to_string() } else { format!("{}/{}", collection, key_or_id) }
                    }
                    let player_param = path.into_inner();
                    let player_id = normalize_id("player", &player_param);
                    match ctrl.usecase.get_player_ratings(&player_id).await {
                        Ok(rows) => Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(rows)),
                        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    }
                }))
                .route("/scheduler/status", web::get().to(|_req: HttpRequest, ctrl: web::Data<RatingsController<C>>| async move {
                    let status = ctrl.scheduler.get_status();
                    Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(status))
                }).wrap(crate::auth::AdminAuthMiddleware { redis: std::sync::Arc::new(redis.clone()), db: std::sync::Arc::new(db.clone()) }))
                .route("/scheduler/trigger", web::post().to(|_req: HttpRequest, query: web::Query<TriggerQuery>, ctrl: web::Data<RatingsController<C>>| async move {
                    let period = query.period.clone();
                    let period_resp = period.clone();
                    match ctrl.scheduler.trigger_recalculation(period).await {
                        Ok(()) => Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(serde_json::json!({"status": "triggered", "period": period_resp}))),
                        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    }
                }).wrap(crate::auth::AdminAuthMiddleware { redis: std::sync::Arc::new(redis.clone()), db: std::sync::Arc::new(db.clone()) }))
                .route("/recalculate/historical", web::post().to(|_req: HttpRequest, ctrl: web::Data<RatingsController<C>>| async move {
                    match ctrl.usecase.recalculate_all_historical_ratings().await {
                        Ok(()) => Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(serde_json::json!({
                            "status": "completed", 
                            "message": "Historical Glicko2 ratings recalculated from 2000 onwards"
                        }))),
                        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
                    }
                }).wrap(crate::auth::AdminAuthMiddleware { redis: std::sync::Arc::new(redis.clone()), db: std::sync::Arc::new(db.clone()) }))
        );
    }

    async fn recompute(&self, _req: HttpRequest, query: RecomputeQuery) -> shared::Result<HttpResponse> {
        let period = query.period.clone();
        self.usecase.recompute_month(period).await?;
        Ok(HttpResponse::Accepted().json(serde_json::json!({"status":"started"})))
    }
}

#[derive(Deserialize)]
struct RecomputeQuery { period: Option<String> }

#[derive(Deserialize)]
struct TriggerQuery { period: Option<String> }

#[derive(Deserialize)]
#[allow(dead_code)]
struct LeaderboardQuery {
    scope: Option<String>,
    min_games: Option<i32>,
    limit: Option<i32>,
}

#[derive(Deserialize)]
struct HistoryQuery {
    scope: Option<String>, // "global" or "game/<id>"
}

#[derive(Deserialize)]
struct DebugPlayerPeriodQuery {
    email: String,
    period: String, // YYYY-MM
}


#[cfg(test)]
mod tests {
    use super::RatingsController;
    use arangors::client::reqwest::ReqwestClient;
    use shared::RatingScope;

    #[test]
    fn normalize_player_key_to_id() {
        let id = RatingsController::<ReqwestClient>::normalize_id("player", "p1");
        assert_eq!(id, "player/p1");
    }

    #[test]
    fn normalize_player_full_id_kept() {
        let id = RatingsController::<ReqwestClient>::normalize_id("player", "player/p1");
        assert_eq!(id, "player/p1");
    }

    fn parse_scope(input: Option<&str>) -> RatingScope {
        match input {
            Some("global") | None => RatingScope::Global,
            Some(s) if s.starts_with("game/") => RatingScope::Game(s.to_string()),
            Some(_) => RatingScope::Global,
        }
    }

    #[test]
    fn scope_defaults_to_global_when_none() {
        assert!(matches!(parse_scope(None), RatingScope::Global));
    }

    #[test]
    fn scope_parses_global_string() {
        assert!(matches!(parse_scope(Some("global")), RatingScope::Global));
    }

    #[test]
    fn scope_parses_game_prefix() {
        match parse_scope(Some("game/abc123")) {
            RatingScope::Game(s) => assert_eq!(s, "game/abc123"),
            _ => panic!("expected Game scope"),
        }
    }

    #[test]
    fn scope_unknown_strings_fallback_to_global() {
        assert!(matches!(parse_scope(Some("weird")), RatingScope::Global));
    }

    fn defaults(min_games: Option<i32>, limit: Option<i32>) -> (i32, i32) {
        (min_games.unwrap_or(10), limit.unwrap_or(50))
    }

    #[test]
    fn leaderboard_defaults_applied() {
        let (mg, lim) = defaults(None, None);
        assert_eq!(mg, 10);
        assert_eq!(lim, 50);
    }

    #[test]
    fn leaderboard_respects_overrides() {
        let (mg, lim) = defaults(Some(3), Some(5));
        assert_eq!(mg, 3);
        assert_eq!(lim, 5);
    }
}


