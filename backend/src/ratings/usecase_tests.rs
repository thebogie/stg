#[cfg(test)]
mod ratings_usecase_tests {
    use super::*;
    use crate::ratings::{RatingsUsecase, RatingsRepository, Glicko2Params};
    use shared::dto::ratings::*;
    use shared::Result;
    use chrono::{Utc, Duration};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock repository for testing
    #[derive(Clone)]
    struct MockRatingsRepository {
        player_ratings: Arc<Mutex<Vec<PlayerRatingDto>>>,
        rating_history: Arc<Mutex<Vec<PlayerRatingHistoryPointDto>>>,
        leaderboard_entries: Arc<Mutex<Vec<RatingLeaderboardEntryDto>>>,
        earliest_contest_date: Arc<Mutex<String>>,
    }

    impl MockRatingsRepository {
        fn new() -> Self {
            Self {
                player_ratings: Arc::new(Mutex::new(vec![])),
                rating_history: Arc::new(Mutex::new(vec![])),
                leaderboard_entries: Arc::new(Mutex::new(vec![])),
                earliest_contest_date: Arc::new(Mutex::new("2024-01-01".to_string())),
            }
        }

        fn set_player_ratings(&self, ratings: Vec<PlayerRatingDto>) {
            let mut player_ratings = self.player_ratings.blocking_lock();
            *player_ratings = ratings;
        }

        fn set_rating_history(&self, history: Vec<PlayerRatingHistoryPointDto>) {
            let mut rating_history = self.rating_history.blocking_lock();
            *rating_history = history;
        }

        fn set_leaderboard_entries(&self, entries: Vec<RatingLeaderboardEntryDto>) {
            let mut leaderboard_entries = self.leaderboard_entries.blocking_lock();
            *leaderboard_entries = entries;
        }

        fn set_earliest_contest_date(&self, date: String) {
            let mut earliest_contest_date = self.earliest_contest_date.blocking_lock();
            *earliest_contest_date = date;
        }
    }

    #[async_trait::async_trait]
    impl RatingsRepository<arangors::client::reqwest::ReqwestClient> for MockRatingsRepository {
        async fn get_player_rating(&self, player_id: &str, scope: RatingScope) -> Result<PlayerRatingDto> {
            let ratings = self.player_ratings.lock().await;
            ratings.iter()
                .find(|r| r.player_id == player_id && r.scope == scope)
                .cloned()
                .ok_or_else(|| shared::SharedError::NotFound("Player rating not found".into()))
        }

        async fn get_all_player_ratings(&self, scope: RatingScope) -> Result<Vec<PlayerRatingDto>> {
            let ratings = self.player_ratings.lock().await;
            Ok(ratings.iter()
                .filter(|r| r.scope == scope)
                .cloned()
                .collect())
        }

        async fn get_player_rating_history(&self, player_id: &str, scope: RatingScope) -> Result<Vec<PlayerRatingHistoryPointDto>> {
            let history = self.rating_history.lock().await;
            Ok(history.iter()
                .filter(|h| h.player_id == player_id && h.scope == scope)
                .cloned()
                .collect())
        }

        async fn get_rating_leaderboard(&self, scope: RatingScope, limit: Option<usize>) -> Result<Vec<RatingLeaderboardEntryDto>> {
            let entries = self.leaderboard_entries.lock().await;
            let mut filtered_entries: Vec<RatingLeaderboardEntryDto> = entries.iter()
                .filter(|e| e.scope == scope)
                .cloned()
                .collect();
            
            filtered_entries.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap());
            
            if let Some(limit) = limit {
                filtered_entries.truncate(limit);
            }
            
            Ok(filtered_entries)
        }

        async fn update_player_rating(&self, rating: &PlayerRatingDto) -> Result<()> {
            let mut ratings = self.player_ratings.lock().await;
            if let Some(existing) = ratings.iter_mut().find(|r| r.player_id == rating.player_id && r.scope == rating.scope) {
                *existing = rating.clone();
            } else {
                ratings.push(rating.clone());
            }
            Ok(())
        }

        async fn add_rating_history_point(&self, point: &PlayerRatingHistoryPointDto) -> Result<()> {
            let mut history = self.rating_history.lock().await;
            history.push(point.clone());
            Ok(())
        }

        async fn clear_all_ratings(&self) -> Result<()> {
            let mut ratings = self.player_ratings.lock().await;
            ratings.clear();
            let mut history = self.rating_history.lock().await;
            history.clear();
            Ok(())
        }

        async fn get_earliest_contest_date(&self) -> Result<String> {
            let date = self.earliest_contest_date.lock().await;
            Ok(date.clone())
        }

        async fn recalculate_ratings_for_period(&self, year: i32, month: u32) -> Result<()> {
            // Mock implementation - just return success
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_ratings_usecase_creation() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        // Test that we can access the repository
        let ratings = usecase.repo.get_all_player_ratings(RatingScope::Overall).await.unwrap();
        assert!(ratings.is_empty());
    }

    #[tokio::test]
    async fn test_get_player_rating() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        let rating = PlayerRatingDto {
            player_id: "player/test".to_string(),
            player_handle: "testuser".to_string(),
            scope: RatingScope::Overall,
            rating: 1250.0,
            rd: 200.0,
            vol: 0.06,
            games_played: 10,
            wins: 7,
            losses: 3,
            last_updated: Utc::now().fixed_offset(),
        };
        
        repo.set_player_ratings(vec![rating.clone()]);
        
        let result = usecase.get_player_rating("player/test", RatingScope::Overall).await.unwrap();
        
        assert_eq!(result.player_id, "player/test");
        assert_eq!(result.player_handle, "testuser");
        assert_eq!(result.scope, RatingScope::Overall);
        assert_eq!(result.rating, 1250.0);
        assert_eq!(result.rd, 200.0);
        assert_eq!(result.vol, 0.06);
        assert_eq!(result.games_played, 10);
        assert_eq!(result.wins, 7);
        assert_eq!(result.losses, 3);
    }

    #[tokio::test]
    async fn test_get_player_rating_not_found() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        let result = usecase.get_player_rating("player/nonexistent", RatingScope::Overall).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_all_player_ratings() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        let ratings = vec![
            PlayerRatingDto {
                player_id: "player/1".to_string(),
                player_handle: "player1".to_string(),
                scope: RatingScope::Overall,
                rating: 1250.0,
                rd: 200.0,
                vol: 0.06,
                games_played: 10,
                wins: 7,
                losses: 3,
                last_updated: Utc::now().fixed_offset(),
            },
            PlayerRatingDto {
                player_id: "player/2".to_string(),
                player_handle: "player2".to_string(),
                scope: RatingScope::Overall,
                rating: 1300.0,
                rd: 180.0,
                vol: 0.05,
                games_played: 15,
                wins: 10,
                losses: 5,
                last_updated: Utc::now().fixed_offset(),
            },
        ];
        
        repo.set_player_ratings(ratings.clone());
        
        let result = usecase.get_all_player_ratings(RatingScope::Overall).await.unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].player_handle, "player1");
        assert_eq!(result[1].player_handle, "player2");
    }

    #[tokio::test]
    async fn test_get_player_rating_history() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        let history = vec![
            PlayerRatingHistoryPointDto {
                player_id: "player/test".to_string(),
                player_handle: "testuser".to_string(),
                scope: RatingScope::Overall,
                rating: 1200.0,
                rd: 350.0,
                vol: 0.06,
                games_played: 0,
                wins: 0,
                losses: 0,
                timestamp: Utc::now().fixed_offset() - Duration::days(30),
            },
            PlayerRatingHistoryPointDto {
                player_id: "player/test".to_string(),
                player_handle: "testuser".to_string(),
                scope: RatingScope::Overall,
                rating: 1250.0,
                rd: 200.0,
                vol: 0.06,
                games_played: 10,
                wins: 7,
                losses: 3,
                timestamp: Utc::now().fixed_offset(),
            },
        ];
        
        repo.set_rating_history(history.clone());
        
        let result = usecase.get_player_rating_history("player/test", RatingScope::Overall).await.unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].rating, 1200.0);
        assert_eq!(result[1].rating, 1250.0);
    }

    #[tokio::test]
    async fn test_get_rating_leaderboard() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        let entries = vec![
            RatingLeaderboardEntryDto {
                player_id: "player/1".to_string(),
                player_handle: "player1".to_string(),
                scope: RatingScope::Overall,
                rating: 1300.0,
                rd: 180.0,
                vol: 0.05,
                games_played: 15,
                wins: 10,
                losses: 5,
                rank: 1,
                last_updated: Utc::now().fixed_offset(),
            },
            RatingLeaderboardEntryDto {
                player_id: "player/2".to_string(),
                player_handle: "player2".to_string(),
                scope: RatingScope::Overall,
                rating: 1250.0,
                rd: 200.0,
                vol: 0.06,
                games_played: 10,
                wins: 7,
                losses: 3,
                rank: 2,
                last_updated: Utc::now().fixed_offset(),
            },
        ];
        
        repo.set_leaderboard_entries(entries.clone());
        
        let result = usecase.get_rating_leaderboard(RatingScope::Overall, None).await.unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].player_handle, "player1");
        assert_eq!(result[0].rating, 1300.0);
        assert_eq!(result[1].player_handle, "player2");
        assert_eq!(result[1].rating, 1250.0);
    }

    #[tokio::test]
    async fn test_get_rating_leaderboard_with_limit() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        let entries = vec![
            RatingLeaderboardEntryDto {
                player_id: "player/1".to_string(),
                player_handle: "player1".to_string(),
                scope: RatingScope::Overall,
                rating: 1300.0,
                rd: 180.0,
                vol: 0.05,
                games_played: 15,
                wins: 10,
                losses: 5,
                rank: 1,
                last_updated: Utc::now().fixed_offset(),
            },
            RatingLeaderboardEntryDto {
                player_id: "player/2".to_string(),
                player_handle: "player2".to_string(),
                scope: RatingScope::Overall,
                rating: 1250.0,
                rd: 200.0,
                vol: 0.06,
                games_played: 10,
                wins: 7,
                losses: 3,
                rank: 2,
                last_updated: Utc::now().fixed_offset(),
            },
        ];
        
        repo.set_leaderboard_entries(entries.clone());
        
        let result = usecase.get_rating_leaderboard(RatingScope::Overall, Some(1)).await.unwrap();
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].player_handle, "player1");
        assert_eq!(result[0].rating, 1300.0);
    }

    #[tokio::test]
    async fn test_get_rating_leaderboard_different_scopes() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        let entries = vec![
            RatingLeaderboardEntryDto {
                player_id: "player/1".to_string(),
                player_handle: "player1".to_string(),
                scope: RatingScope::Overall,
                rating: 1300.0,
                rd: 180.0,
                vol: 0.05,
                games_played: 15,
                wins: 10,
                losses: 5,
                rank: 1,
                last_updated: Utc::now().fixed_offset(),
            },
            RatingLeaderboardEntryDto {
                player_id: "player/2".to_string(),
                player_handle: "player2".to_string(),
                scope: RatingScope::Game("game/test".to_string()),
                rating: 1250.0,
                rd: 200.0,
                vol: 0.06,
                games_played: 10,
                wins: 7,
                losses: 3,
                rank: 1,
                last_updated: Utc::now().fixed_offset(),
            },
        ];
        
        repo.set_leaderboard_entries(entries.clone());
        
        // Test Overall scope
        let overall_result = usecase.get_rating_leaderboard(RatingScope::Overall, None).await.unwrap();
        assert_eq!(overall_result.len(), 1);
        assert_eq!(overall_result[0].scope, RatingScope::Overall);
        
        // Test Game scope
        let game_result = usecase.get_rating_leaderboard(RatingScope::Game("game/test".to_string()), None).await.unwrap();
        assert_eq!(game_result.len(), 1);
        assert_eq!(game_result[0].scope, RatingScope::Game("game/test".to_string()));
    }

    #[tokio::test]
    async fn test_update_player_rating() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        let rating = PlayerRatingDto {
            player_id: "player/test".to_string(),
            player_handle: "testuser".to_string(),
            scope: RatingScope::Overall,
            rating: 1250.0,
            rd: 200.0,
            vol: 0.06,
            games_played: 10,
            wins: 7,
            losses: 3,
            last_updated: Utc::now().fixed_offset(),
        };
        
        let result = usecase.update_player_rating(&rating).await;
        assert!(result.is_ok());
        
        // Verify the rating was stored
        let stored_rating = usecase.get_player_rating("player/test", RatingScope::Overall).await.unwrap();
        assert_eq!(stored_rating.rating, 1250.0);
    }

    #[tokio::test]
    async fn test_add_rating_history_point() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        let history_point = PlayerRatingHistoryPointDto {
            player_id: "player/test".to_string(),
            player_handle: "testuser".to_string(),
            scope: RatingScope::Overall,
            rating: 1250.0,
            rd: 200.0,
            vol: 0.06,
            games_played: 10,
            wins: 7,
            losses: 3,
            timestamp: Utc::now().fixed_offset(),
        };
        
        let result = usecase.add_rating_history_point(&history_point).await;
        assert!(result.is_ok());
        
        // Verify the history point was stored
        let history = usecase.get_player_rating_history("player/test", RatingScope::Overall).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].rating, 1250.0);
    }

    #[tokio::test]
    async fn test_clear_all_ratings() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        // Add some test data
        let rating = PlayerRatingDto {
            player_id: "player/test".to_string(),
            player_handle: "testuser".to_string(),
            scope: RatingScope::Overall,
            rating: 1250.0,
            rd: 200.0,
            vol: 0.06,
            games_played: 10,
            wins: 7,
            losses: 3,
            last_updated: Utc::now().fixed_offset(),
        };
        
        usecase.update_player_rating(&rating).await.unwrap();
        
        // Verify data exists
        let ratings = usecase.get_all_player_ratings(RatingScope::Overall).await.unwrap();
        assert_eq!(ratings.len(), 1);
        
        // Clear all ratings
        let result = usecase.clear_all_ratings().await;
        assert!(result.is_ok());
        
        // Verify data is cleared
        let ratings = usecase.get_all_player_ratings(RatingScope::Overall).await.unwrap();
        assert_eq!(ratings.len(), 0);
    }

    #[tokio::test]
    async fn test_get_earliest_contest_date() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        repo.set_earliest_contest_date("2024-01-15".to_string());
        
        let date = usecase.get_earliest_contest_date().await.unwrap();
        assert_eq!(date, "2024-01-15");
    }

    #[tokio::test]
    async fn test_recalculate_ratings_for_period() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        let result = usecase.recalculate_ratings_for_period(2024, 1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_recalculate_all_historical_ratings() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        repo.set_earliest_contest_date("2024-01-01".to_string());
        
        let result = usecase.recalculate_all_historical_ratings().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_recalculate_all_historical_ratings_invalid_date() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        repo.set_earliest_contest_date("invalid-date".to_string());
        
        let result = usecase.recalculate_all_historical_ratings().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_usecase_clone() {
        let repo = MockRatingsRepository::new();
        let usecase1 = RatingsUsecase::new(repo);
        let usecase2 = usecase1.clone();
        
        // Both should be able to access the same repository
        let ratings1 = usecase1.get_all_player_ratings(RatingScope::Overall).await.unwrap();
        let ratings2 = usecase2.get_all_player_ratings(RatingScope::Overall).await.unwrap();
        
        assert_eq!(ratings1.len(), ratings2.len());
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let repo = MockRatingsRepository::new();
        let usecase = RatingsUsecase::new(repo);
        
        // Test concurrent access to different methods
        let (ratings_result, leaderboard_result, history_result) = tokio::join!(
            usecase.get_all_player_ratings(RatingScope::Overall),
            usecase.get_rating_leaderboard(RatingScope::Overall, None),
            usecase.get_player_rating_history("player/test", RatingScope::Overall)
        );
        
        assert!(ratings_result.is_ok());
        assert!(leaderboard_result.is_ok());
        assert!(history_result.is_ok());
    }

    #[tokio::test]
    async fn test_glicko2_params_default() {
        let params = Glicko2Params::default();
        assert!(params.tau > 0.0);
        assert!(params.epsilon > 0.0);
    }

    #[tokio::test]
    async fn test_rating_scope_enum() {
        // Test Overall scope
        match RatingScope::Overall {
            RatingScope::Overall => assert!(true),
            _ => assert!(false),
        }
        
        // Test Game scope
        match RatingScope::Game("game/test".to_string()) {
            RatingScope::Game(game_id) => assert_eq!(game_id, "game/test"),
            _ => assert!(false),
        }
    }
}
