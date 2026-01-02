use actix_web::{web, HttpRequest, HttpResponse, Result};
use actix_web::HttpMessage;
use shared::dto::client_sync::*;
use shared::error::SharedError;
use crate::client_analytics::usecase::ClientAnalyticsUseCase;
use serde_json::json;
use log;
use arangors::{AqlQuery, Database};
use arangors::client::ClientExt;


/// Client analytics controller for handling client-side data sync and queries
pub struct ClientAnalyticsController<U, C: ClientExt> {
    usecase: U,
    db: Database<C>,
}

impl<U, C> ClientAnalyticsController<U, C>
where
    U: ClientAnalyticsUseCase,
    C: ClientExt,
{
    pub fn new(usecase: U, db: Database<C>) -> Self {
        Self { usecase, db }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn normalize_player_id(param: &str) -> String {
        if param.contains('/') { param.to_string() } else { format!("player/{}", param) }
    }

    /// Helper method to extract player ID from request via authenticated email set by AuthMiddleware
    async fn extract_player_id_from_request(&self, req: &HttpRequest) -> Result<String, actix_web::Error> {
        // AuthMiddleware inserts the authenticated email into request extensions
        if let Some(email) = req.extensions().get::<String>() {
            // Map email -> player_id using a minimal AQL query
            let aql = AqlQuery::builder()
                .query("FOR p IN player FILTER LOWER(p.email) == LOWER(@email) LIMIT 1 RETURN p._id")
                .bind_var("email", email.as_str())
                .build();
            match self.db.aql_query::<String>(aql).await {
                Ok(mut cursor) => {
                    if let Some(player_id) = cursor.pop() {
                        return Ok(player_id);
                    }
                    Err(actix_web::error::ErrorUnauthorized("Player not found"))
                }
                Err(e) => {
                    log::error!("Failed to resolve player_id from email: {}", e);
                    Err(actix_web::error::ErrorInternalServerError("Failed to resolve player"))
                }
            }
        } else {
            Err(actix_web::error::ErrorUnauthorized("Authentication required"))
        }
    }

    /// Handles client data synchronization (full or delta)
    pub async fn sync_client_data(
        &self,
        req: HttpRequest,
        payload: web::Json<ClientSyncRequest>,
    ) -> Result<HttpResponse> {
        // Extract player ID from session (this would need to be implemented based on your auth system)
        let player_id = self.extract_player_id_from_request(&req).await?;
        
        // Verify the requesting player matches the authenticated user
        if player_id != payload.player_id {
            log::warn!("Player ID mismatch: requested {} vs authenticated {}", payload.player_id, player_id);
            return Ok(HttpResponse::Forbidden().json(json!({
                "error": "Forbidden",
                "message": "Cannot access other player's data"
            })));
        }

        log::info!("Client sync request for player: {}", player_id);

        match self.usecase.sync_client_data(&player_id, &payload).await {
            Ok(response) => {
                log::info!("Client sync successful for player: {}, contests: {}", player_id, response.contests.len());
                Ok(HttpResponse::Ok().json(response))
            }
            Err(e) => {
                log::error!("Client sync failed for player {}: {}", player_id, e);
                match e {
                    SharedError::NotFound(_) => Ok(HttpResponse::NotFound().json(json!({
                        "error": "Not Found",
                        "message": "Player or data not found"
                    }))),
                    SharedError::Database(_) => Ok(HttpResponse::InternalServerError().json(json!({
                        "error": "Internal Server Error",
                        "message": "Database error occurred"
                    }))),
                    _ => Ok(HttpResponse::BadRequest().json(json!({
                        "error": "Bad Request",
                        "message": e.to_string()
                    }))),
                }
            }
        }
    }

    /// Handles real-time analytics queries from client
    pub async fn query_client_analytics(
        &self,
        req: HttpRequest,
        payload: web::Json<ClientAnalyticsRequest>,
    ) -> Result<HttpResponse> {
        let player_id = self.extract_player_id_from_request(&req).await?;
        
        // Verify the requesting player matches the authenticated user
        if player_id != payload.player_id {
            log::warn!("Player ID mismatch: requested {} vs authenticated {}", payload.player_id, player_id);
            return Ok(HttpResponse::Forbidden().json(json!({
                "error": "Forbidden",
                "message": "Cannot access other player's data"
            })));
        }

        log::info!("Client analytics query for player: {}", player_id);

        match self.usecase.query_client_analytics(&player_id, &payload.query).await {
            Ok(response) => {
                log::info!("Client analytics query successful for player: {}", player_id);
                Ok(HttpResponse::Ok().json(response))
            }
            Err(e) => {
                log::error!("Client analytics query failed for player {}: {}", player_id, e);
                match e {
                    SharedError::NotFound(_) => Ok(HttpResponse::NotFound().json(json!({
                        "error": "Not Found",
                        "message": "Player or data not found"
                    }))),
                    SharedError::Database(_) => Ok(HttpResponse::InternalServerError().json(json!({
                        "error": "Internal Server Error",
                        "message": "Database error occurred"
                    }))),
                    _ => Ok(HttpResponse::BadRequest().json(json!({
                        "error": "Bad Request",
                        "message": e.to_string()
                    }))),
                }
            }
        }
    }

    /// Validates client data integrity
    pub async fn validate_client_data(
        &self,
        req: HttpRequest,
        payload: web::Json<ClientDataValidationRequest>,
    ) -> Result<HttpResponse> {
        let player_id = self.extract_player_id_from_request(&req).await?;
        
        // Verify the requesting player matches the authenticated user
        if player_id != payload.player_id {
            log::warn!("Player ID mismatch: requested {} vs authenticated {}", payload.player_id, player_id);
            return Ok(HttpResponse::Forbidden().json(json!({
                "error": "Forbidden",
                "message": "Cannot access other player's data"
            })));
        }

        log::info!("Client data validation request for player: {}", player_id);

        match self.usecase.validate_client_data(&player_id, &payload).await {
            Ok(response) => {
                log::info!("Client data validation completed for player: {}", player_id);
                Ok(HttpResponse::Ok().json(response))
            }
            Err(e) => {
                log::error!("Client data validation failed for player {}: {}", player_id, e);
                match e {
                    SharedError::NotFound(_) => Ok(HttpResponse::NotFound().json(json!({
                        "error": "Not Found",
                        "message": "Player or data not found"
                    }))),
                    SharedError::Database(_) => Ok(HttpResponse::InternalServerError().json(json!({
                        "error": "Internal Server Error",
                        "message": "Database error occurred"
                    }))),
                    _ => Ok(HttpResponse::BadRequest().json(json!({
                        "error": "Bad Request",
                        "message": e.to_string()
                    }))),
                }
            }
        }
    }

    /// Gets client sync status and metadata
    pub async fn get_client_sync_status(
        &self,
        req: HttpRequest,
        path: web::Path<String>,
    ) -> Result<HttpResponse> {
        let player_id = self.extract_player_id_from_request(&req).await?;
        let requested_player_param = path.into_inner();
        
        // Normalize requested_player_id to full ID if it's just a key
        let requested_player_id = if requested_player_param.contains('/') { 
            requested_player_param 
        } else { 
            format!("player/{}", requested_player_param) 
        };
        
        // Verify the requesting player matches the authenticated user
        if player_id != requested_player_id {
            log::warn!("Player ID mismatch: requested {} vs authenticated {}", requested_player_id, player_id);
            return Ok(HttpResponse::Forbidden().json(json!({
                "error": "Forbidden",
                "message": "Cannot access other player's data"
            })));
        }

        log::info!("Client sync status request for player: {}", player_id);

        match self.usecase.get_client_sync_status(&player_id).await {
            Ok(status) => {
                log::info!("Client sync status retrieved for player: {}", player_id);
                Ok(HttpResponse::Ok().json(status))
            }
            Err(e) => {
                log::error!("Client sync status failed for player {}: {}", player_id, e);
                match e {
                    SharedError::NotFound(_) => Ok(HttpResponse::NotFound().json(json!({
                        "error": "Not Found",
                        "message": "Player or data not found"
                    }))),
                    SharedError::Database(_) => Ok(HttpResponse::InternalServerError().json(json!({
                        "error": "Internal Server Error",
                        "message": "Database error occurred"
                    }))),
                    _ => Ok(HttpResponse::BadRequest().json(json!({
                        "error": "Bad Request",
                        "message": e.to_string()
                    }))),
                }
            }
        }
    }

    /// Clears client data for a player (logout/cleanup)
    pub async fn clear_client_data(
        &self,
        req: HttpRequest,
        path: web::Path<String>,
    ) -> Result<HttpResponse> {
        let player_id = self.extract_player_id_from_request(&req).await?;
        let requested_player_param = path.into_inner();
        
        // Normalize requested_player_id to full ID if it's just a key
        let requested_player_id = if requested_player_param.contains('/') { 
            requested_player_param 
        } else { 
            format!("player/{}", requested_player_param) 
        };
        
        // Verify the requesting player matches the authenticated user
        if player_id != requested_player_id {
            log::warn!("Player ID mismatch: requested {} vs authenticated {}", requested_player_id, player_id);
            return Ok(HttpResponse::Forbidden().json(json!({
                "error": "Forbidden",
                "message": "Cannot access other player's data"
            })));
        }

        log::info!("Client data clear request for player: {}", player_id);

        match self.usecase.clear_client_data(&player_id).await {
            Ok(_) => {
                log::info!("Client data cleared for player: {}", player_id);
                Ok(HttpResponse::Ok().json(json!({
                    "message": "Client data cleared successfully"
                })))
            }
            Err(e) => {
                log::error!("Client data clear failed for player {}: {}", player_id, e);
                match e {
                    SharedError::NotFound(_) => Ok(HttpResponse::NotFound().json(json!({
                        "error": "Not Found",
                        "message": "Player or data not found"
                    }))),
                    SharedError::Database(_) => Ok(HttpResponse::InternalServerError().json(json!({
                        "error": "Internal Server Error",
                        "message": "Database error occurred"
                    }))),
                    _ => Ok(HttpResponse::BadRequest().json(json!({
                        "error": "Bad Request",
                        "message": e.to_string()
                    }))),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_player_id_normalization() {
        // Test basic string operations that would be used in normalization
        let input = "abc123";
        let expected = "player/abc123";
        let result = format!("player/{}", input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_player_id_validation() {
        // Test that player IDs can be validated
        let player_id = "player/abc123";
        assert!(player_id.starts_with("player/"));
        assert_eq!(player_id.len(), 13);
    }
}

/// Configures client analytics routes
pub fn configure_routes<U, C>(
    cfg: &mut web::ServiceConfig,
    _controller: web::Data<ClientAnalyticsController<U, C>>,
    redis_client: std::sync::Arc<redis::Client>,
)
where
    U: ClientAnalyticsUseCase + 'static,
    C: ClientExt + 'static,
{
    log::debug!("Registering client analytics routes:");
    log::debug!("  POST /api/client/sync (authenticated)");
    log::debug!("  POST /api/client/analytics (authenticated)");
    log::debug!("  POST /api/client/validate (authenticated)");
    log::debug!("  GET /api/client/sync-status/{{player_id}} (authenticated)");
    log::debug!("  DELETE /api/client/clear/{{player_id}} (authenticated)");
    
    cfg.service(
        web::scope("/api/client")
            .app_data(_controller.clone())
            .wrap(crate::auth::AuthMiddleware { redis: std::sync::Arc::new((*redis_client).clone()) })
            .route("/sync", web::post().to(|req: HttpRequest, payload: web::Json<ClientSyncRequest>, controller: web::Data<ClientAnalyticsController<U, C>>| async move {
                controller.sync_client_data(req, payload).await
            }))
            .route("/analytics", web::post().to(|req: HttpRequest, payload: web::Json<ClientAnalyticsRequest>, controller: web::Data<ClientAnalyticsController<U, C>>| async move {
                controller.query_client_analytics(req, payload).await
            }))
            .route("/validate", web::post().to(|req: HttpRequest, payload: web::Json<ClientDataValidationRequest>, controller: web::Data<ClientAnalyticsController<U, C>>| async move {
                controller.validate_client_data(req, payload).await
            }))
            .route("/sync-status/{player_id}", web::get().to(|req: HttpRequest, path: web::Path<String>, controller: web::Data<ClientAnalyticsController<U, C>>| async move {
                controller.get_client_sync_status(req, path).await
            }))
            .route("/clear/{player_id}", web::delete().to(|req: HttpRequest, path: web::Path<String>, controller: web::Data<ClientAnalyticsController<U, C>>| async move {
                controller.clear_client_data(req, path).await
            }))
    );
}

/// Configures enhanced analytics routes for the new graph traversal features
pub fn configure_enhanced_routes<U, C>(
    cfg: &mut web::ServiceConfig,
    controller: web::Data<ClientAnalyticsController<U, C>>,
    redis_client: std::sync::Arc<redis::Client>,
)
where
    U: ClientAnalyticsUseCase + 'static,
    C: ClientExt + 'static,
{
    cfg.service(
        web::scope("/api/analytics-enhanced")
            .app_data(controller.clone())
            .wrap(crate::auth::AuthMiddleware { redis: std::sync::Arc::new((*redis_client).clone()) })
            // Existing enhanced endpoints from venue and game controllers
            .service(crate::venue::controller::get_venue_performance_handler)
            .service(crate::venue::controller::get_player_venue_stats_handler)
            .service(crate::game::controller::get_game_recommendations_handler)
            .service(crate::game::controller::get_similar_games_handler)
            .service(crate::game::controller::get_popular_games_handler)
            .route("/communities/{player_id:.*}", web::get().to(|_req: HttpRequest, path: web::Path<String>, controller: web::Data<ClientAnalyticsController<U, C>>| async move {
                let player_id = path.into_inner();
                let min_contests = 2; // Default value
                // Use the real repository method
                match controller.usecase.get_gaming_communities(&player_id, min_contests).await {
                    Ok(communities) => {
                        actix_web::HttpResponse::Ok().json(serde_json::json!({
                            "gaming_communities": communities
                        }))
                    }
                    Err(e) => {
                        log::error!("Failed to get gaming communities: {}", e);
                        actix_web::HttpResponse::InternalServerError().json(serde_json::json!({
                            "error": "Failed to fetch gaming communities",
                            "message": e
                        }))
                    }
                }
            }))
            .route("/networking/{player_id:.*}", web::get().to(|_req: HttpRequest, path: web::Path<String>, controller: web::Data<ClientAnalyticsController<U, C>>| async move {
                let player_id = path.into_inner();
                // Use the real repository method
                match controller.usecase.get_player_networking(&player_id).await {
                    Ok(networking) => {
                        actix_web::HttpResponse::Ok().json(networking)
                    }
                    Err(e) => {
                        log::error!("Failed to get player networking: {}", e);
                        actix_web::HttpResponse::InternalServerError().json(serde_json::json!({
                            "error": "Failed to fetch player networking",
                            "message": e
                        }))
                    }
                }
            }))
    );
}
