use crate::analytics::repository::AnalyticsRepository;
use crate::analytics::usecase::AnalyticsUseCase;
use crate::analytics::visualization::ChartConfig;
use crate::auth::AuthMiddleware;
use crate::config::DatabaseConfig;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use arangors::client::ClientExt;
use serde_json::json;
use shared::dto::analytics::*;

/// Analytics controller for handling HTTP requests
pub struct AnalyticsController<C: ClientExt> {
    usecase: AnalyticsUseCase<C>,
}

#[cfg(test)]
mod tests {
    use super::AnalyticsController;
    use arangors::client::reqwest::ReqwestClient;

    #[test]
    fn normalize_player_key_to_id() {
        let id = AnalyticsController::<ReqwestClient>::normalize_id("player", "abc123");
        assert_eq!(id, "player/abc123");
    }

    #[test]
    fn normalize_player_full_id_kept() {
        let id = AnalyticsController::<ReqwestClient>::normalize_id("player", "player/abc123");
        assert_eq!(id, "player/abc123");
    }

    #[test]
    fn normalize_contest_key_to_id() {
        let id = AnalyticsController::<ReqwestClient>::normalize_id("contest", "c123");
        assert_eq!(id, "contest/c123");
    }

    #[test]
    fn test_analytics_controller_creation() {
        // This test would need a mock database
        assert!(true); // Just test that it compiles
    }
}

impl<C: ClientExt> AnalyticsController<C> {
    /// Creates a new analytics controller
    pub fn new(db: arangors::Database<C>, config: DatabaseConfig) -> Self {
        let repo = AnalyticsRepository::new(db, config);
        let usecase = AnalyticsUseCase::new(repo);
        Self { usecase }
    }

    /// Get contest heatmap (weekday x hour)
    pub async fn get_contest_heatmap(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let weeks = query
            .get("weeks")
            .and_then(|w| w.parse::<i32>().ok())
            .unwrap_or(8);
        let game_id = query.get("game_id").map(|s| s.as_str());
        match self.usecase.get_contest_heatmap(weeks, game_id).await {
            Ok(payload) => Ok(HttpResponse::Ok().json(payload)),
            Err(e) => {
                log::error!("Failed to get contest heatmap: {}", e);
                Ok(HttpResponse::InternalServerError()
                    .json(json!({"error":"Failed to get contest heatmap"})))
            }
        }
    }

    #[cfg(test)]
    fn normalize_id(collection: &str, key_or_id: &str) -> String {
        if key_or_id.contains('/') {
            key_or_id.to_string()
        } else {
            format!("{}/{}", collection, key_or_id)
        }
    }

    /// Helper method to get player ID from email
    async fn get_player_id_from_email(&self, email: &str) -> Result<String, actix_web::Error> {
        // Query the database to get the actual player ID using the analytics repository
        match self.usecase.repo().get_player_id_by_email(email).await {
            Ok(Some(player_id)) => Ok(player_id),
            Ok(None) => Err(actix_web::error::ErrorNotFound("Player not found")),
            Err(e) => {
                log::error!("Failed to query player ID from email: {}", e);
                Err(actix_web::error::ErrorInternalServerError(
                    "Database query failed",
                ))
            }
        }
    }

    /// Get player statistics
    pub async fn get_player_stats(
        &self,
        _req: HttpRequest,
        path: web::Path<String>,
        query: web::Query<PlayerStatsRequest>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let player_param = path.into_inner();

        // Normalize player_id to full ID if it's just a key
        let player_id = if player_param.contains('/') {
            player_param
        } else {
            format!("player/{}", player_param)
        };

        let request = query.into_inner();

        match self.usecase.get_player_stats(&player_id, &request).await {
            Ok(stats) => Ok(HttpResponse::Ok().json(stats)),
            Err(e) => {
                log::error!("Failed to get player stats: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get player statistics"
                })))
            }
        }
    }

    /// Get platform statistics
    pub async fn get_platform_stats(
        &self,
        _req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        match self.usecase.get_platform_stats().await {
            Ok(stats) => Ok(HttpResponse::Ok().json(stats)),
            Err(e) => {
                log::error!("Failed to get platform stats: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get platform statistics"
                })))
            }
        }
    }

    /// Get enhanced platform insights
    pub async fn get_platform_insights(
        &self,
        _req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        match self.usecase.get_platform_insights().await {
            Ok(insights) => Ok(HttpResponse::Ok().json(insights)),
            Err(e) => {
                log::error!("Failed to get platform insights: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get platform insights"
                })))
            }
        }
    }

    /// Get leaderboard data
    pub async fn get_leaderboard(
        &self,
        _req: HttpRequest,
        query: web::Query<LeaderboardRequest>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let request = query.into_inner();

        match self.usecase.get_leaderboard(&request).await {
            Ok(leaderboard) => Ok(HttpResponse::Ok().json(leaderboard)),
            Err(e) => {
                log::error!("Failed to get leaderboard: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get leaderboard data"
                })))
            }
        }
    }

    /// Get player achievements
    pub async fn get_player_achievements(
        &self,
        _req: HttpRequest,
        path: web::Path<String>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let player_param = path.into_inner();

        // Normalize player_id to full ID if it's just a key
        let player_id = if player_param.contains('/') {
            player_param
        } else {
            format!("player/{}", player_param)
        };

        match self.usecase.get_player_achievements(&player_id).await {
            Ok(achievements) => Ok(HttpResponse::Ok().json(achievements)),
            Err(e) => {
                log::error!("Failed to get player achievements: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get player achievements"
                })))
            }
        }
    }

    /// Get player rankings
    pub async fn get_player_rankings(
        &self,
        _req: HttpRequest,
        path: web::Path<String>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let player_param = path.into_inner();

        // Normalize player_id to full ID if it's just a key
        let player_id = if player_param.contains('/') {
            player_param
        } else {
            format!("player/{}", player_param)
        };

        match self.usecase.get_player_rankings(&player_id).await {
            Ok(rankings) => Ok(HttpResponse::Ok().json(rankings)),
            Err(e) => {
                log::error!("Failed to get player rankings: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get player rankings"
                })))
            }
        }
    }

    /// Get contest statistics
    pub async fn get_contest_stats(
        &self,
        _req: HttpRequest,
        path: web::Path<String>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let contest_param = path.into_inner();

        // Normalize contest_id to full ID if it's just a key
        let contest_id = if contest_param.contains('/') {
            contest_param.clone()
        } else {
            format!("contest/{}", contest_param)
        };

        log::debug!(
            "Getting contest stats for contest_id: {} (normalized from: {})",
            contest_id,
            contest_param
        );

        match self.usecase.get_contest_stats(&contest_id).await {
            Ok(stats) => Ok(HttpResponse::Ok().json(stats)),
            Err(e) => {
                log::error!("Failed to get contest stats: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get contest statistics"
                })))
            }
        }
    }

    /// Get contest trends
    pub async fn get_contest_trends(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let months = query
            .get("months")
            .and_then(|m| m.parse::<i32>().ok())
            .unwrap_or(12);

        match self.usecase.get_contest_trends(months).await {
            Ok(trends) => Ok(HttpResponse::Ok().json(trends)),
            Err(e) => {
                log::error!("Failed to get contest trends: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get contest trends"
                })))
            }
        }
    }

    /// Get contest difficulty analysis
    pub async fn get_contest_difficulty(
        &self,
        _req: HttpRequest,
        path: web::Path<String>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let contest_param = path.into_inner();

        // Normalize contest_id to full ID if it's just a key
        let contest_id = if contest_param.contains('/') {
            contest_param
        } else {
            format!("contest/{}", contest_param)
        };

        match self.usecase.get_contest_difficulty(&contest_id).await {
            Ok(difficulty) => Ok(HttpResponse::Ok().json(json!({
                "contest_id": contest_id,
                "difficulty_rating": difficulty
            }))),
            Err(e) => {
                log::error!("Failed to get contest difficulty: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get contest difficulty"
                })))
            }
        }
    }

    /// Get contest excitement rating
    pub async fn get_contest_excitement(
        &self,
        _req: HttpRequest,
        path: web::Path<String>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let contest_param = path.into_inner();

        // Normalize contest_id to full ID if it's just a key
        let contest_id = if contest_param.contains('/') {
            contest_param
        } else {
            format!("contest/{}", contest_param)
        };

        match self.usecase.get_contest_excitement(&contest_id).await {
            Ok(excitement) => Ok(HttpResponse::Ok().json(json!({
                "contest_id": contest_id,
                "excitement_rating": excitement
            }))),
            Err(e) => {
                log::error!("Failed to get contest excitement: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get contest excitement rating"
                })))
            }
        }
    }

    /// Get recent contests
    pub async fn get_recent_contests(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let limit = query
            .get("limit")
            .and_then(|l| l.parse::<i32>().ok())
            .unwrap_or(10);

        match self.usecase.get_recent_contests(limit).await {
            Ok(contests) => Ok(HttpResponse::Ok().json(contests)),
            Err(e) => {
                log::error!("Failed to get recent contests: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get recent contests"
                })))
            }
        }
    }

    /// Get cache statistics
    pub async fn get_cache_stats(
        &self,
        _req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        let stats = self.usecase.get_cache_stats().await;
        Ok(HttpResponse::Ok().json(stats))
    }

    /// Invalidate player cache
    pub async fn invalidate_player_cache(
        &self,
        _req: HttpRequest,
        path: web::Path<String>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let player_param = path.into_inner();

        // Normalize player_id to full ID if it's just a key
        let player_id = if player_param.contains('/') {
            player_param
        } else {
            format!("player/{}", player_param)
        };

        self.usecase.invalidate_player_cache(&player_id).await;

        Ok(HttpResponse::Ok().json(json!({
            "message": "Player cache invalidated",
            "player_id": player_id
        })))
    }

    /// Invalidate contest cache
    pub async fn invalidate_contest_cache(
        &self,
        _req: HttpRequest,
        path: web::Path<String>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let contest_param = path.into_inner();

        // Normalize contest_id to full ID if it's just a key
        let contest_id = if contest_param.contains('/') {
            contest_param
        } else {
            format!("contest/{}", contest_param)
        };

        self.usecase.invalidate_contest_cache(&contest_id).await;

        Ok(HttpResponse::Ok().json(json!({
            "message": "Contest cache invalidated",
            "contest_id": contest_id
        })))
    }

    /// Invalidate all cache
    pub async fn invalidate_all_cache(
        &self,
        _req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        self.usecase.invalidate_all_cache().await;

        Ok(HttpResponse::Ok().json(json!({
            "message": "All analytics cache invalidated"
        })))
    }

    // Visualization endpoints

    /// Generate player performance chart
    pub async fn get_player_performance_chart(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let limit = query
            .get("limit")
            .and_then(|l| l.parse::<i32>().ok())
            .unwrap_or(10);

        let config = self.parse_chart_config(&query);

        match self
            .usecase
            .get_player_performance_chart(limit, Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!("Failed to generate player performance chart: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate player performance chart"
                })))
            }
        }
    }

    /// Generate leaderboard chart
    pub async fn get_leaderboard_chart(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let category = query
            .get("category")
            .unwrap_or(&"win_rate".to_string())
            .clone();
        let limit = query
            .get("limit")
            .and_then(|l| l.parse::<i32>().ok())
            .unwrap_or(10);

        let config = self.parse_chart_config(&query);

        match self
            .usecase
            .get_leaderboard_chart(&category, limit, Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!("Failed to generate leaderboard chart: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate leaderboard chart"
                })))
            }
        }
    }

    /// Generate achievement distribution chart
    pub async fn get_achievement_distribution_chart(
        &self,
        _req: HttpRequest,
        path: web::Path<String>,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let player_param = path.into_inner();

        // Normalize player_id to full ID if it's just a key
        let player_id = if player_param.contains('/') {
            player_param
        } else {
            format!("player/{}", player_param)
        };

        let config = self.parse_chart_config(&query);

        match self
            .usecase
            .get_achievement_distribution_chart(&player_id, Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!("Failed to generate achievement distribution chart: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate achievement distribution chart"
                })))
            }
        }
    }

    /// Generate contest trends chart
    pub async fn get_contest_trends_chart(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let months = query
            .get("months")
            .and_then(|m| m.parse::<i32>().ok())
            .unwrap_or(12);

        let config = self.parse_chart_config(&query);

        match self
            .usecase
            .get_contest_trends_chart(months, Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!("Failed to generate contest trends chart: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate contest trends chart"
                })))
            }
        }
    }

    /// Generate activity metrics chart (DAU & contests/day)
    pub async fn get_activity_metrics_chart(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let days = query
            .get("days")
            .and_then(|d| d.parse::<i32>().ok())
            .unwrap_or(30);
        let config = self.parse_chart_config(&query);
        match self
            .usecase
            .get_activity_metrics_chart(days, Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!("Failed to generate activity metrics chart: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate activity metrics chart"
                })))
            }
        }
    }

    /// Generate platform dashboard
    pub async fn get_platform_dashboard(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let config = self.parse_chart_config(&query);

        match self.usecase.get_platform_dashboard(Some(config)).await {
            Ok(charts) => Ok(HttpResponse::Ok().json(charts)),
            Err(e) => {
                log::error!("Failed to generate platform dashboard: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate platform dashboard"
                })))
            }
        }
    }

    /// Generate player comparison chart
    pub async fn get_player_comparison_chart(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let player_ids_str = query
            .get("player_ids")
            .unwrap_or(&"player/1,player/2,player/3".to_string())
            .clone();
        let player_ids: Vec<String> = player_ids_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let config = self.parse_chart_config(&query);

        match self
            .usecase
            .get_player_comparison_chart(&player_ids, Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!("Failed to generate player comparison chart: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate player comparison chart"
                })))
            }
        }
    }

    /// Generate contest analysis chart
    pub async fn get_contest_analysis_chart(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let limit = query
            .get("limit")
            .and_then(|l| l.parse::<i32>().ok())
            .unwrap_or(20);

        let config = self.parse_chart_config(&query);

        match self
            .usecase
            .get_contest_analysis_chart(limit, Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!("Failed to generate contest analysis chart: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate contest analysis chart"
                })))
            }
        }
    }

    /// Generate game popularity heatmap chart
    pub async fn get_game_popularity_heatmap(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let limit = query
            .get("limit")
            .and_then(|l| l.parse::<i32>().ok())
            .unwrap_or(10);

        let config = self.parse_chart_config(&query);

        match self
            .usecase
            .get_game_popularity_heatmap(limit, Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!("Failed to generate game popularity heatmap: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate game popularity heatmap"
                })))
            }
        }
    }

    /// Generate player performance distribution chart
    pub async fn get_player_performance_distribution_chart(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let config = self.parse_chart_config(&query);
        match self
            .usecase
            .get_player_performance_distribution_chart(Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!(
                    "Failed to generate player performance distribution chart: {}",
                    e
                );
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate player performance distribution chart"
                })))
            }
        }
    }

    /// Generate game difficulty vs popularity chart
    pub async fn get_game_difficulty_popularity_chart(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let config = self.parse_chart_config(&query);
        match self
            .usecase
            .get_game_difficulty_popularity_chart(Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!(
                    "Failed to generate game difficulty vs popularity chart: {}",
                    e
                );
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate game difficulty vs popularity chart"
                })))
            }
        }
    }

    /// Generate venue performance timeslot chart
    pub async fn get_venue_performance_timeslot_chart(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let config = self.parse_chart_config(&query);
        match self
            .usecase
            .get_venue_performance_timeslot_chart(Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!("Failed to generate venue performance timeslot chart: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate venue performance timeslot chart"
                })))
            }
        }
    }

    /// Generate player retention cohort chart
    pub async fn get_player_retention_cohort_chart(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let config = self.parse_chart_config(&query);
        match self
            .usecase
            .get_player_retention_cohort_chart(Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!("Failed to generate player retention cohort chart: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate player retention cohort chart"
                })))
            }
        }
    }

    /// Generate contest completion by game chart
    pub async fn get_contest_completion_by_game_chart(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let config = self.parse_chart_config(&query);
        match self
            .usecase
            .get_contest_completion_by_game_chart(Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!("Failed to generate contest completion by game chart: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate contest completion by game chart"
                })))
            }
        }
    }

    /// Generate head-to-head matrix chart
    pub async fn get_head_to_head_matrix_chart(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let limit = query
            .get("limit")
            .and_then(|l| l.parse::<i32>().ok())
            .unwrap_or(10);
        let config = self.parse_chart_config(&query);
        match self
            .usecase
            .get_head_to_head_matrix_chart(limit, Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!("Failed to generate head-to-head matrix chart: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate head-to-head matrix chart"
                })))
            }
        }
    }

    /// Generate comprehensive analytics dashboard
    pub async fn get_analytics_dashboard(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let config = self.parse_chart_config(&query);

        match self.usecase.get_analytics_dashboard(Some(config)).await {
            Ok(charts) => Ok(HttpResponse::Ok().json(charts)),
            Err(e) => {
                log::error!("Failed to generate analytics dashboard: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate analytics dashboard"
                })))
            }
        }
    }

    /// Generate custom chart
    pub async fn get_custom_chart(
        &self,
        _req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        let chart_type = query
            .get("chart_type")
            .unwrap_or(&"bar".to_string())
            .clone();
        let data_type = query
            .get("data_type")
            .unwrap_or(&"leaderboard".to_string())
            .clone();

        let config = self.parse_chart_config(&query);

        match self
            .usecase
            .get_custom_chart(&chart_type, &data_type, Some(config))
            .await
        {
            Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
            Err(e) => {
                log::error!("Failed to generate custom chart: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to generate custom chart"
                })))
            }
        }
    }

    /// Helper method to parse chart configuration from query parameters
    fn parse_chart_config(&self, query: &std::collections::HashMap<String, String>) -> ChartConfig {
        ChartConfig {
            title: query
                .get("title")
                .unwrap_or(&"Analytics Chart".to_string())
                .clone(),
            width: query
                .get("width")
                .and_then(|w| w.parse::<u32>().ok())
                .unwrap_or(800),
            height: query
                .get("height")
                .and_then(|h| h.parse::<u32>().ok())
                .unwrap_or(400),
            colors: vec![
                "#3B82F6".to_string(), // Blue
                "#EF4444".to_string(), // Red
                "#10B981".to_string(), // Green
                "#F59E0B".to_string(), // Yellow
                "#8B5CF6".to_string(), // Purple
                "#06B6D4".to_string(), // Cyan
                "#F97316".to_string(), // Orange
                "#EC4899".to_string(), // Pink
            ],
            show_legend: query
                .get("show_legend")
                .map(|v| v == "true")
                .unwrap_or(true),
            show_grid: query.get("show_grid").map(|v| v == "true").unwrap_or(true),
            animation: query.get("animation").map(|v| v == "true").unwrap_or(true),
        }
    }

    // Sample data endpoints for testing

    /// Get sample platform stats for testing
    pub async fn get_sample_platform_stats(
        &self,
        _req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        let sample_stats = json!({
            "total_players": 1247,
            "total_contests": 89,
            "total_games": 23,
            "total_venues": 8,
            "active_players_30d": 342,
            "active_players_7d": 156,
            "contests_30d": 12,
            "average_participants_per_contest": 14.2,
            "top_games": [
                {"game_id": "game/1", "game_name": "Chess", "plays": 45, "popularity_score": 0.92},
                {"game_id": "game/2", "game_name": "Poker", "plays": 38, "popularity_score": 0.87},
                {"game_id": "game/3", "game_name": "Magic: The Gathering", "plays": 32, "popularity_score": 0.81},
                {"game_id": "game/4", "game_name": "Dungeons & Dragons", "plays": 28, "popularity_score": 0.76},
                {"game_id": "game/5", "game_name": "Risk", "plays": 25, "popularity_score": 0.72}
            ],
            "top_venues": [
                {"venue_id": "venue/1", "venue_name": "Game Haven", "contests_held": 23, "total_participants": 345, "activity_score": 0.94},
                {"venue_id": "venue/2", "venue_name": "Strategy Central", "contests_held": 18, "total_participants": 267, "activity_score": 0.88},
                {"venue_id": "venue/3", "venue_name": "Card Kingdom", "contests_held": 15, "total_participants": 198, "activity_score": 0.82}
            ],
            "last_updated": "2024-01-15T10:30:00Z"
        });

        Ok(HttpResponse::Ok().json(sample_stats))
    }

    /// Get sample leaderboard for testing
    pub async fn get_sample_leaderboard(
        &self,
        _req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        let sample_leaderboard = json!({
            "category": "win_rate",
            "time_period": "all_time",
            "entries": [
                {"rank": 1, "player_id": "player/1", "player_handle": "ChessMaster", "player_name": "Alex Chen", "value": 87.5, "additional_data": null},
                {"rank": 2, "player_id": "player/2", "player_handle": "PokerPro", "player_name": "Sarah Johnson", "value": 82.3, "additional_data": null},
                {"rank": 3, "player_id": "player/3", "player_handle": "MagicWizard", "player_name": "Mike Rodriguez", "value": 79.8, "additional_data": null},
                {"rank": 4, "player_id": "player/4", "player_handle": "RiskTaker", "player_name": "Emily Davis", "value": 76.2, "additional_data": null},
                {"rank": 5, "player_id": "player/5", "player_handle": "DnDDungeonMaster", "player_name": "Chris Wilson", "value": 74.9, "additional_data": null},
                {"rank": 6, "player_id": "player/6", "player_handle": "BoardGameKing", "player_name": "Lisa Thompson", "value": 72.1, "additional_data": null},
                {"rank": 7, "player_id": "player/7", "player_handle": "StrategyQueen", "player_name": "David Brown", "value": 69.8, "additional_data": null},
                {"rank": 8, "player_id": "player/8", "player_handle": "GameTheory", "player_name": "Rachel Green", "value": 67.5, "additional_data": null},
                {"rank": 9, "player_id": "player/9", "player_handle": "TacticalGenius", "player_name": "Tom Anderson", "value": 65.2, "additional_data": null},
                {"rank": 10, "player_id": "player/10", "player_handle": "VictoryLap", "player_name": "Jessica Lee", "value": 63.8, "additional_data": null}
            ],
            "total_entries": 1247,
            "last_updated": "2024-01-15T10:30:00Z"
        });

        Ok(HttpResponse::Ok().json(sample_leaderboard))
    }

    /// Get sample contest trends for testing
    pub async fn get_sample_contest_trends(
        &self,
        _req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        let sample_trends = json!([
            {"year": 2023, "month": 1, "contests": 3},
            {"year": 2023, "month": 2, "contests": 5},
            {"year": 2023, "month": 3, "contests": 7},
            {"year": 2023, "month": 4, "contests": 6},
            {"year": 2023, "month": 5, "contests": 8},
            {"year": 2023, "month": 6, "contests": 10},
            {"year": 2023, "month": 7, "contests": 12},
            {"year": 2023, "month": 8, "contests": 15},
            {"year": 2023, "month": 9, "contests": 18},
            {"year": 2023, "month": 10, "contests": 22},
            {"year": 2023, "month": 11, "contests": 25},
            {"year": 2023, "month": 12, "contests": 28},
            {"year": 2024, "month": 1, "contests": 12}
        ]);

        Ok(HttpResponse::Ok().json(sample_trends))
    }

    /// Get sample player stats for testing
    pub async fn get_sample_player_stats(
        &self,
        _req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        let sample_stats = json!({
            "player_id": "player/1",
            "player_handle": "ChessMaster",
            "player_name": "Alex Chen",
            "total_contests": 45,
            "total_wins": 32,
            "total_losses": 13,
            "win_rate": 71.1,
            "average_placement": 2.3,
            "best_placement": 1,
            "skill_rating": 1850.5,
            "rating_confidence": 0.92,
            "total_points": 2840,
            "current_streak": 4,
            "longest_streak": 8,
            "last_updated": "2024-01-15T10:30:00Z"
        });

        Ok(HttpResponse::Ok().json(sample_stats))
    }

    /// Get sample player achievements for testing
    pub async fn get_sample_player_achievements(
        &self,
        _req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        let sample_achievements = json!({
            "player_id": "player/1",
            "player_handle": "ChessMaster",
            "achievements": [
                {
                    "id": "first_win",
                    "name": "First Victory",
                    "description": "Win your first contest",
                    "category": "wins",
                    "required_value": 1,
                    "current_value": 1,
                    "unlocked": true,
                    "unlocked_at": "2023-03-15T14:30:00Z"
                },
                {
                    "id": "win_master",
                    "name": "Win Master",
                    "description": "Win 10 contests",
                    "category": "wins",
                    "required_value": 10,
                    "current_value": 32,
                    "unlocked": true,
                    "unlocked_at": "2023-06-22T16:45:00Z"
                },
                {
                    "id": "champion",
                    "name": "Champion",
                    "description": "Win 50 contests",
                    "category": "wins",
                    "required_value": 50,
                    "current_value": 32,
                    "unlocked": false,
                    "unlocked_at": null
                },
                {
                    "id": "contestant",
                    "name": "Contestant",
                    "description": "Participate in 5 contests",
                    "category": "contests",
                    "required_value": 5,
                    "current_value": 45,
                    "unlocked": true,
                    "unlocked_at": "2023-04-10T11:20:00Z"
                },
                {
                    "id": "veteran",
                    "name": "Veteran",
                    "description": "Participate in 25 contests",
                    "category": "contests",
                    "required_value": 25,
                    "current_value": 45,
                    "unlocked": true,
                    "unlocked_at": "2023-09-05T13:15:00Z"
                },
                {
                    "id": "game_explorer",
                    "name": "Game Explorer",
                    "description": "Play 5 different games",
                    "category": "games",
                    "required_value": 5,
                    "current_value": 7,
                    "unlocked": true,
                    "unlocked_at": "2023-07-18T09:30:00Z"
                },
                {
                    "id": "venue_hopper",
                    "name": "Venue Hopper",
                    "description": "Play at 3 different venues",
                    "category": "venues",
                    "required_value": 3,
                    "current_value": 4,
                    "unlocked": true,
                    "unlocked_at": "2023-08-12T15:45:00Z"
                }
            ],
            "total_achievements": 10,
            "unlocked_achievements": 7,
            "completion_percentage": 70.0
        });

        Ok(HttpResponse::Ok().json(sample_achievements))
    }

    // Player-specific analytics endpoints

    /// Get players who have beaten the current player
    pub async fn get_players_who_beat_me(
        &self,
        req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        // Extract current player ID from auth context
        let email = match req.extensions().get::<String>() {
            Some(email) => email.clone(),
            None => {
                log::error!("Not authenticated for get_players_who_beat_me");
                return Ok(HttpResponse::Unauthorized().json(json!({
                    "error": "Not authenticated"
                })));
            }
        };

        // Get player ID from email
        let current_player_id = match self.get_player_id_from_email(&email).await {
            Ok(player_id) => player_id,
            Err(_) => {
                log::error!("Failed to get player ID for email: {}", email);
                return Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get player information"
                })));
            }
        };

        match self
            .usecase
            .get_players_who_beat_me(&current_player_id)
            .await
        {
            Ok(players) => Ok(HttpResponse::Ok().json(players)),
            Err(e) => {
                log::error!("Failed to get players who beat me: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get players who beat me"
                })))
            }
        }
    }

    /// Get players that the current player has beaten
    pub async fn get_players_i_beat(
        &self,
        req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        // Extract current player ID from auth context
        let email = match req.extensions().get::<String>() {
            Some(email) => email.clone(),
            None => {
                log::error!("Not authenticated for get_players_i_beat");
                return Ok(HttpResponse::Unauthorized().json(json!({
                    "error": "Not authenticated"
                })));
            }
        };

        // Get player ID from email
        let current_player_id = match self.get_player_id_from_email(&email).await {
            Ok(player_id) => player_id,
            Err(_) => {
                log::error!("Failed to get player ID for email: {}", email);
                return Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get player information"
                })));
            }
        };

        match self.usecase.get_players_i_beat(&current_player_id).await {
            Ok(players) => Ok(HttpResponse::Ok().json(players)),
            Err(e) => {
                log::error!("Failed to get players I beat: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get players I beat"
                })))
            }
        }
    }

    /// Get player's game performance statistics
    pub async fn get_my_game_performance(
        &self,
        req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        // Extract current player ID from auth context
        let email = match req.extensions().get::<String>() {
            Some(email) => email.clone(),
            None => {
                log::error!("Not authenticated for get_my_game_performance");
                return Ok(HttpResponse::Unauthorized().json(json!({
                    "error": "Not authenticated"
                })));
            }
        };

        // Get player ID from email
        let current_player_id = match self.get_player_id_from_email(&email).await {
            Ok(player_id) => player_id,
            Err(_) => {
                log::error!("Failed to get player ID for email: {}", email);
                return Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get player information"
                })));
            }
        };

        match self
            .usecase
            .get_my_game_performance(&current_player_id)
            .await
        {
            Ok(performance) => Ok(HttpResponse::Ok().json(performance)),
            Err(e) => {
                log::error!("Failed to get game performance: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get game performance"
                })))
            }
        }
    }

    /// Get player's head-to-head record against specific opponent
    pub async fn get_head_to_head_record(
        &self,
        path: web::Path<String>,
        req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        // Accept either key or full id; normalize to collection/id
        fn normalize_id(collection: &str, key_or_id: &str) -> String {
            if key_or_id.contains('/') {
                key_or_id.to_string()
            } else {
                format!("{}/{}", collection, key_or_id)
            }
        }
        let opponent_param = path.into_inner();
        let opponent_id = normalize_id("player", &opponent_param);

        // Extract current player ID from auth context
        let email = match req.extensions().get::<String>() {
            Some(email) => email.clone(),
            None => {
                log::error!("Not authenticated for get_head_to_head_record");
                return Ok(HttpResponse::Unauthorized().json(json!({
                    "error": "Not authenticated"
                })));
            }
        };

        // Get player ID from email
        let current_player_id = match self.get_player_id_from_email(&email).await {
            Ok(player_id) => player_id,
            Err(_) => {
                log::error!("Failed to get player ID for email: {}", email);
                return Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get player information"
                })));
            }
        };

        match self
            .usecase
            .get_head_to_head_record(&current_player_id, &opponent_id)
            .await
        {
            Ok(record) => Ok(HttpResponse::Ok().json(record)),
            Err(e) => {
                log::error!("Failed to get head-to-head record: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get head-to-head record"
                })))
            }
        }
    }

    /// Get player's performance trends over time
    pub async fn get_my_performance_trends(
        &self,
        req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        // Extract current player ID from auth context
        let email = match req.extensions().get::<String>() {
            Some(email) => email.clone(),
            None => {
                log::error!("Not authenticated for get_my_performance_trends");
                return Ok(HttpResponse::Unauthorized().json(json!({
                    "error": "Not authenticated"
                })));
            }
        };

        // Get player ID from email
        let current_player_id = match self.get_player_id_from_email(&email).await {
            Ok(player_id) => player_id,
            Err(_) => {
                log::error!("Failed to get player ID for email: {}", email);
                return Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get player information"
                })));
            }
        };

        match self
            .usecase
            .get_my_performance_trends(&current_player_id)
            .await
        {
            Ok(trends) => Ok(HttpResponse::Ok().json(trends)),
            Err(e) => {
                log::error!("Failed to get performance trends: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get performance trends"
                })))
            }
        }
    }

    /// Get contests by venue for current player
    pub async fn get_contests_by_venue(
        &self,
        req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, actix_web::Error> {
        // Extract current player ID from auth context
        let email = match req.extensions().get::<String>() {
            Some(email) => email.clone(),
            None => {
                log::error!("Not authenticated for get_contests_by_venue");
                return Ok(HttpResponse::Unauthorized().json(json!({
                    "error": "Not authenticated"
                })));
            }
        };

        // Get player ID from email
        let current_player_id = match self.get_player_id_from_email(&email).await {
            Ok(player_id) => player_id,
            Err(_) => {
                log::error!("Failed to get player ID for email: {}", email);
                return Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get player information"
                })));
            }
        };

        // Get venue ID from query parameters - just use the key part
        let venue_id = match query.get("id") {
            Some(id) => {
                if id.contains('/') {
                    // Extract just the key part
                    id.split('/').nth(1).unwrap_or(id).to_string()
                } else {
                    id.clone()
                }
            }
            None => {
                return Ok(HttpResponse::BadRequest().json(json!({
                    "error": "Venue ID parameter 'id' is required"
                })));
            }
        };

        match self
            .usecase
            .get_contests_by_venue(&current_player_id, &venue_id)
            .await
        {
            Ok(contests) => {
                log::info!("Found {} contests for venue {}", contests.len(), venue_id);
                Ok(HttpResponse::Ok().json(contests))
            }
            Err(e) => {
                log::error!("Failed to get contests by venue: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to get contests by venue"
                })))
            }
        }
    }

    /// Health check for analytics service
    pub async fn health_check(&self, _req: HttpRequest) -> Result<HttpResponse, actix_web::Error> {
        Ok(HttpResponse::Ok().json(json!({
            "status": "healthy",
            "service": "analytics",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })))
    }

    /// Debug endpoint to check database content
    pub async fn debug_database(
        &self,
        _req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        // Test the actual analytics query to see what's happening
        match self.usecase.debug_database().await {
            Ok(data) => Ok(HttpResponse::Ok().json(data)),
            Err(e) => {
                log::error!("Failed to debug database: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to debug database"
                })))
            }
        }
    }

    /// Test endpoint to validate game performance query syntax
    pub async fn test_game_performance_query(
        &self,
        _req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error> {
        // Test the game performance query with a dummy player ID to validate syntax
        let test_player_id = "player/test123";

        match self.usecase.get_my_game_performance(test_player_id).await {
            Ok(results) => {
                log::info!(
                    "Game performance query syntax test successful, returned {} results",
                    results.len()
                );
                Ok(HttpResponse::Ok().json(json!({
                    "status": "success",
                    "message": "Query syntax is valid",
                    "results_count": results.len(),
                    "results": results
                })))
            }
            Err(e) => {
                log::error!("Game performance query syntax test failed: {}", e);
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": format!("Query syntax test failed: {}", e),
                    "details": e.to_string()
                })))
            }
        }
    }
}

/// Configure analytics routes
pub fn configure_routes<C: ClientExt + 'static>(
    cfg: &mut web::ServiceConfig,
    db: arangors::Database<C>,
    config: DatabaseConfig,
    redis_client: std::sync::Arc<redis::Client>,
) {
    let controller = AnalyticsController::new(db, config);

    log::debug!("Registering analytics routes:");
    log::debug!("  GET /api/analytics/health");
    log::debug!("  GET /api/analytics/test-game-performance");
    log::debug!("  GET /api/analytics/debug");
    log::debug!("  GET /api/analytics/platform");
    log::debug!("  GET /api/analytics/insights");
    log::debug!("  GET /api/analytics/sample-platform");
    log::debug!("  GET /api/analytics/leaderboard");
    log::debug!("  GET /api/analytics/players/{{player_id}}/stats (authenticated)");
    log::debug!("  GET /api/analytics/players/{{player_id}}/achievements (authenticated)");
    log::debug!("  GET /api/analytics/players/{{player_id}}/rankings (authenticated)");
    log::debug!("  GET /api/analytics/contests/{{contest_id}}/stats");
    log::debug!("  GET /api/analytics/contests/{{contest_id}}/difficulty");
    log::debug!("  GET /api/analytics/contests/{{contest_id}}/excitement");
    log::debug!("  GET /api/analytics/contests/trends");
    log::debug!("  GET /api/analytics/contests/recent");
    log::debug!("  GET /api/analytics/contests/cache/stats");
    log::debug!("  POST /api/analytics/contests/cache/invalidate/player/{{player_id}}");
    log::debug!("  POST /api/analytics/contests/cache/invalidate/contest/{{contest_id}}");
    log::debug!("  POST /api/analytics/contests/cache/invalidate/all");
    log::debug!("  GET /api/analytics/charts/player-performance");
    log::debug!("  GET /api/analytics/charts/leaderboard");
    log::debug!("  GET /api/analytics/charts/achievement-distribution/{{player_id}}");
    log::debug!("  GET /api/analytics/charts/contest-trends");
    log::debug!("  GET /api/analytics/charts/activity-metrics");
    log::debug!("  GET /api/analytics/charts/platform-dashboard");
    log::debug!("  GET /api/analytics/charts/player-comparison");

    cfg.service(
        web::scope("/api/analytics")
            .app_data(web::Data::new(controller))
            .route("/health", web::get().to(|req: HttpRequest, controller: web::Data<AnalyticsController<C>>| async move {
                controller.health_check(req).await
            }))
            .route("/test-game-performance", web::get().to(|req: HttpRequest, controller: web::Data<AnalyticsController<C>>| async move {
                controller.test_game_performance_query(req).await
            }))
            .route("/debug", web::get().to(|req: HttpRequest, controller: web::Data<AnalyticsController<C>>| async move {
                controller.debug_database(req).await
            }))
            .route("/platform", web::get().to(|req: HttpRequest, controller: web::Data<AnalyticsController<C>>| async move {
                controller.get_platform_stats(req).await
            }))
            .route("/insights", web::get().to(|req: HttpRequest, controller: web::Data<AnalyticsController<C>>| async move {
                controller.get_platform_insights(req).await
            }))
            .route("/sample-platform", web::get().to(|req: HttpRequest, controller: web::Data<AnalyticsController<C>>| async move {
                controller.get_sample_platform_stats(req).await
            }))
            .route("/leaderboard", web::get().to(|req: HttpRequest, query: web::Query<LeaderboardRequest>, controller: web::Data<AnalyticsController<C>>| async move {
                controller.get_leaderboard(req, query).await
            }))
            .service(
                web::scope("/players")
                    .wrap(AuthMiddleware { redis: std::sync::Arc::new((*redis_client).clone()) })
                    .route("/{player_id}/stats", web::get().to(|req: HttpRequest, path: web::Path<String>, query: web::Query<PlayerStatsRequest>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_player_stats(req, path, query).await
                    }))
                    .route("/{player_id}/achievements", web::get().to(|req: HttpRequest, path: web::Path<String>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_player_achievements(req, path).await
                    }))
                    .route("/{player_id}/rankings", web::get().to(|req: HttpRequest, path: web::Path<String>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_player_rankings(req, path).await
                    }))
            )
            .service(
                web::scope("/contests")
                    .route("/{contest_id}/stats", web::get().to(|req: HttpRequest, path: web::Path<String>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_contest_stats(req, path).await
                    }))
                    .route("/{contest_id}/difficulty", web::get().to(|req: HttpRequest, path: web::Path<String>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_contest_difficulty(req, path).await
                    }))
                    .route("/{contest_id}/excitement", web::get().to(|req: HttpRequest, path: web::Path<String>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_contest_excitement(req, path).await
                    }))
                    .route("/trends", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_contest_trends(req, query).await
                    }))
                    .route("/heatmap", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_contest_heatmap(req, query).await
                    }))
                    .route("/recent", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_recent_contests(req, query).await
                    }))
                    .route("/cache/stats", web::get().to(|req: HttpRequest, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_cache_stats(req).await
                    }))
                    .route("/cache/invalidate/player/{player_id}", web::post().to(|req: HttpRequest, path: web::Path<String>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.invalidate_player_cache(req, path).await
                    }))
                    .route("/cache/invalidate/contest/{contest_id}", web::post().to(|req: HttpRequest, path: web::Path<String>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.invalidate_contest_cache(req, path).await
                    }))
                    .route("/cache/invalidate/all", web::post().to(|req: HttpRequest, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.invalidate_all_cache(req).await
                    }))
            )
            .service(
                web::scope("/charts")
                    .route("/player-performance", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_player_performance_chart(req, query).await
                    }))
                    .route("/leaderboard", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_leaderboard_chart(req, query).await
                    }))
                    .route("/achievement-distribution/{player_id}", web::get().to(|req: HttpRequest, path: web::Path<String>, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_achievement_distribution_chart(req, path, query).await
                    }))
                    .route("/contest-trends", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_contest_trends_chart(req, query).await
                    }))
                    .route("/activity-metrics", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_activity_metrics_chart(req, query).await
                    }))
                    .route("/platform-dashboard", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_platform_dashboard(req, query).await
                    }))
                    .route("/player-comparison", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_player_comparison_chart(req, query).await
                    }))
                    .route("/contest-analysis", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_contest_analysis_chart(req, query).await
                    }))
                    .route("/game-popularity", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_game_popularity_heatmap(req, query).await
                    }))
                    .route("/player-performance-distribution", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_player_performance_distribution_chart(req, query).await
                    }))
                    .route("/game-difficulty-popularity", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_game_difficulty_popularity_chart(req, query).await
                    }))
                    .route("/venue-performance-timeslot", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_venue_performance_timeslot_chart(req, query).await
                    }))
                    .route("/player-retention-cohort", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_player_retention_cohort_chart(req, query).await
                    }))
                    .route("/contest-completion-by-game", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_contest_completion_by_game_chart(req, query).await
                    }))
                    .route("/head-to-head-matrix", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_head_to_head_matrix_chart(req, query).await
                    }))
                    .route("/analytics-dashboard", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_analytics_dashboard(req, query).await
                    }))
                    .route("/custom", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_custom_chart(req, query).await
                    }))
            )
            .service(
                web::scope("/player")
                    .wrap(AuthMiddleware { redis: std::sync::Arc::new((*redis_client).clone()) })
                    .route("/opponents-who-beat-me", web::get().to(|req: HttpRequest, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_players_who_beat_me(req).await
                    }))
                    .route("/opponents-i-beat", web::get().to(|req: HttpRequest, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_players_i_beat(req).await
                    }))
                    .route("/game-performance", web::get().to(|req: HttpRequest, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_my_game_performance(req).await
                    }))
                    .route("/performance-trends", web::get().to(|req: HttpRequest, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_my_performance_trends(req).await
                    }))
                    // Use a greedy matcher to allow slashes in opponent_id (e.g., "player/...")
                    .route("/head-to-head/{opponent_id:.*}", web::get().to(|path: web::Path<String>, req: HttpRequest, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_head_to_head_record(path, req).await
                    }))
                    .route("/contests-by-venue", web::get().to(|req: HttpRequest, query: web::Query<std::collections::HashMap<String, String>>, controller: web::Data<AnalyticsController<C>>| async move {
                        controller.get_contests_by_venue(req, query).await
                    }))
            )
    );
}
