#[cfg(test)]
mod ratings_scheduler_tests {
    use super::*;
    use crate::ratings::{RatingsScheduler, RatingsUsecase, Glicko2Params};
    use shared::Result;
    use chrono::{Utc, Duration};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use std::time::Duration as StdDuration;

    // Mock usecase for testing
    #[derive(Clone)]
    struct MockRatingsUsecase {
        recalculate_called: Arc<Mutex<bool>>,
        recalculate_period_called: Arc<Mutex<Vec<(i32, u32)>>>,
        should_fail: Arc<Mutex<bool>>,
    }

    impl MockRatingsUsecase {
        fn new() -> Self {
            Self {
                recalculate_called: Arc::new(Mutex::new(false)),
                recalculate_period_called: Arc::new(Mutex::new(vec![])),
                should_fail: Arc::new(Mutex::new(false)),
            }
        }

        fn set_should_fail(&self, fail: bool) {
            let mut should_fail = self.should_fail.blocking_lock();
            *should_fail = fail;
        }

        fn was_recalculate_called(&self) -> bool {
            let recalculate_called = self.recalculate_called.blocking_lock();
            *recalculate_called
        }

        fn get_recalculate_period_calls(&self) -> Vec<(i32, u32)> {
            let recalculate_period_called = self.recalculate_period_called.blocking_lock();
            recalculate_period_called.clone()
        }
    }

    #[async_trait::async_trait]
    impl RatingsUsecase<arangors::client::reqwest::ReqwestClient> for MockRatingsUsecase {
        async fn recalculate_all_historical_ratings(&self) -> Result<()> {
            let mut recalculate_called = self.recalculate_called.lock().await;
            *recalculate_called = true;
            
            let should_fail = self.should_fail.lock().await;
            if *should_fail {
                Err(shared::SharedError::InternalError("Mock failure".into()))
            } else {
                Ok(())
            }
        }

        async fn recalculate_ratings_for_period(&self, year: i32, month: u32) -> Result<()> {
            let mut recalculate_period_called = self.recalculate_period_called.lock().await;
            recalculate_period_called.push((year, month));
            
            let should_fail = self.should_fail.lock().await;
            if *should_fail {
                Err(shared::SharedError::InternalError("Mock failure".into()))
            } else {
                Ok(())
            }
        }

        async fn get_player_rating(&self, _player_id: &str, _scope: shared::dto::ratings::RatingScope) -> Result<shared::dto::ratings::PlayerRatingDto> {
            Err(shared::SharedError::NotFound("Not implemented in mock".into()))
        }

        async fn get_all_player_ratings(&self, _scope: shared::dto::ratings::RatingScope) -> Result<Vec<shared::dto::ratings::PlayerRatingDto>> {
            Ok(vec![])
        }

        async fn get_player_rating_history(&self, _player_id: &str, _scope: shared::dto::ratings::RatingScope) -> Result<Vec<shared::dto::ratings::PlayerRatingHistoryPointDto>> {
            Ok(vec![])
        }

        async fn get_rating_leaderboard(&self, _scope: shared::dto::ratings::RatingScope, _limit: Option<usize>) -> Result<Vec<shared::dto::ratings::RatingLeaderboardEntryDto>> {
            Ok(vec![])
        }

        async fn update_player_rating(&self, _rating: &shared::dto::ratings::PlayerRatingDto) -> Result<()> {
            Ok(())
        }

        async fn add_rating_history_point(&self, _point: &shared::dto::ratings::PlayerRatingHistoryPointDto) -> Result<()> {
            Ok(())
        }

        async fn clear_all_ratings(&self) -> Result<()> {
            Ok(())
        }

        async fn get_earliest_contest_date(&self) -> Result<String> {
            Ok("2024-01-01".to_string())
        }
    }

    #[tokio::test]
    async fn test_ratings_scheduler_creation() {
        let usecase = MockRatingsUsecase::new();
        let scheduler = RatingsScheduler::new(usecase);
        
        assert!(!scheduler.is_running);
        assert!(scheduler.last_run.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn test_ratings_scheduler_start() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        let result = scheduler.start().await;
        assert!(result.is_ok());
        assert!(scheduler.is_running);
    }

    #[tokio::test]
    async fn test_ratings_scheduler_start_already_running() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Start first time
        let result1 = scheduler.start().await;
        assert!(result1.is_ok());
        assert!(scheduler.is_running);
        
        // Try to start again
        let result2 = scheduler.start().await;
        assert!(result2.is_ok()); // Should not error, just warn
        assert!(scheduler.is_running);
    }

    #[tokio::test]
    async fn test_ratings_scheduler_stop() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Start scheduler
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running);
        
        // Stop scheduler
        scheduler.stop();
        assert!(!scheduler.is_running);
    }

    #[tokio::test]
    async fn test_ratings_scheduler_stop_not_running() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Stop scheduler that's not running
        scheduler.stop();
        assert!(!scheduler.is_running);
    }

    #[tokio::test]
    async fn test_ratings_scheduler_get_status() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Test status when not running
        let status = scheduler.get_status();
        assert_eq!(status.is_running, false);
        assert!(status.last_run.is_none());
        assert!(status.next_run.is_none());
        
        // Start scheduler
        scheduler.start().await.unwrap();
        
        // Test status when running
        let status = scheduler.get_status();
        assert_eq!(status.is_running, true);
        assert!(status.last_run.is_none()); // No runs yet
        assert!(status.next_run.is_some());
    }

    #[tokio::test]
    async fn test_ratings_scheduler_get_status_with_last_run() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Set a last run time
        let last_run = Utc::now();
        {
            let mut last_run_mutex = scheduler.last_run.lock().unwrap();
            *last_run_mutex = Some(last_run);
        }
        
        // Start scheduler
        scheduler.start().await.unwrap();
        
        // Test status
        let status = scheduler.get_status();
        assert_eq!(status.is_running, true);
        assert!(status.last_run.is_some());
        assert!(status.next_run.is_some());
    }

    #[tokio::test]
    async fn test_ratings_scheduler_manual_trigger() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Start scheduler
        scheduler.start().await.unwrap();
        
        // Trigger manual recalculation
        let result = scheduler.trigger_manual_recalculation().await;
        assert!(result.is_ok());
        
        // Check that recalculate was called
        assert!(usecase.was_recalculate_called());
    }

    #[tokio::test]
    async fn test_ratings_scheduler_manual_trigger_not_running() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Try to trigger manual recalculation without starting
        let result = scheduler.trigger_manual_recalculation().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ratings_scheduler_manual_trigger_failure() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Set usecase to fail
        usecase.set_should_fail(true);
        
        // Start scheduler
        scheduler.start().await.unwrap();
        
        // Trigger manual recalculation
        let result = scheduler.trigger_manual_recalculation().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ratings_scheduler_trigger_period_recalculation() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Start scheduler
        scheduler.start().await.unwrap();
        
        // Trigger period recalculation
        let result = scheduler.trigger_period_recalculation(2024, 1).await;
        assert!(result.is_ok());
        
        // Check that recalculate_period was called
        let calls = usecase.get_recalculate_period_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], (2024, 1));
    }

    #[tokio::test]
    async fn test_ratings_scheduler_trigger_period_recalculation_not_running() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Try to trigger period recalculation without starting
        let result = scheduler.trigger_period_recalculation(2024, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ratings_scheduler_trigger_period_recalculation_failure() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Set usecase to fail
        usecase.set_should_fail(true);
        
        // Start scheduler
        scheduler.start().await.unwrap();
        
        // Trigger period recalculation
        let result = scheduler.trigger_period_recalculation(2024, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ratings_scheduler_multiple_period_calls() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Start scheduler
        scheduler.start().await.unwrap();
        
        // Trigger multiple period recalculations
        scheduler.trigger_period_recalculation(2024, 1).await.unwrap();
        scheduler.trigger_period_recalculation(2024, 2).await.unwrap();
        scheduler.trigger_period_recalculation(2024, 3).await.unwrap();
        
        // Check that all calls were made
        let calls = usecase.get_recalculate_period_calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0], (2024, 1));
        assert_eq!(calls[1], (2024, 2));
        assert_eq!(calls[2], (2024, 3));
    }

    #[tokio::test]
    async fn test_ratings_scheduler_clone() {
        let usecase = MockRatingsUsecase::new();
        let scheduler1 = RatingsScheduler::new(usecase);
        let scheduler2 = scheduler1.clone();
        
        // Both should have the same initial state
        assert_eq!(scheduler1.is_running, scheduler2.is_running);
        assert_eq!(scheduler1.last_run.lock().unwrap().is_none(), scheduler2.last_run.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn test_ratings_scheduler_status_serialization() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Start scheduler
        scheduler.start().await.unwrap();
        
        // Get status
        let status = scheduler.get_status();
        
        // Test that status can be serialized
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("is_running"));
        assert!(json.contains("true"));
    }

    #[tokio::test]
    async fn test_ratings_scheduler_status_with_timestamps() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Set a last run time
        let last_run = Utc::now();
        {
            let mut last_run_mutex = scheduler.last_run.lock().unwrap();
            *last_run_mutex = Some(last_run);
        }
        
        // Start scheduler
        scheduler.start().await.unwrap();
        
        // Get status
        let status = scheduler.get_status();
        
        // Test that status contains timestamps
        assert!(status.last_run.is_some());
        assert!(status.next_run.is_some());
        
        // Test that timestamps are reasonable
        let now = Utc::now();
        if let Some(last_run) = status.last_run {
            assert!(last_run <= now);
        }
        if let Some(next_run) = status.next_run {
            assert!(next_run > now);
        }
    }

    #[tokio::test]
    async fn test_ratings_scheduler_concurrent_access() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Start scheduler
        scheduler.start().await.unwrap();
        
        // Test concurrent access to status
        let (status1, status2) = tokio::join!(
            async { scheduler.get_status() },
            async { scheduler.get_status() }
        );
        
        assert_eq!(status1.is_running, status2.is_running);
        assert_eq!(status1.last_run, status2.last_run);
    }

    #[tokio::test]
    async fn test_ratings_scheduler_error_handling() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Set usecase to fail
        usecase.set_should_fail(true);
        
        // Start scheduler
        scheduler.start().await.unwrap();
        
        // Try operations that should fail
        let result1 = scheduler.trigger_manual_recalculation().await;
        assert!(result1.is_err());
        
        let result2 = scheduler.trigger_period_recalculation(2024, 1).await;
        assert!(result2.is_err());
        
        // Status should still be available
        let status = scheduler.get_status();
        assert_eq!(status.is_running, true);
    }

    #[tokio::test]
    async fn test_ratings_scheduler_lifecycle() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Initial state
        assert!(!scheduler.is_running);
        let status = scheduler.get_status();
        assert_eq!(status.is_running, false);
        
        // Start scheduler
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running);
        let status = scheduler.get_status();
        assert_eq!(status.is_running, true);
        
        // Stop scheduler
        scheduler.stop();
        assert!(!scheduler.is_running);
        let status = scheduler.get_status();
        assert_eq!(status.is_running, false);
        
        // Restart scheduler
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running);
        let status = scheduler.get_status();
        assert_eq!(status.is_running, true);
    }

    #[tokio::test]
    async fn test_ratings_scheduler_background_task() {
        let usecase = MockRatingsUsecase::new();
        let mut scheduler = RatingsScheduler::new(usecase);
        
        // Start scheduler
        scheduler.start().await.unwrap();
        
        // Wait a bit to let background task start
        tokio::time::sleep(StdDuration::from_millis(100)).await;
        
        // Check that scheduler is still running
        assert!(scheduler.is_running);
        
        // Stop scheduler
        scheduler.stop();
        assert!(!scheduler.is_running);
    }
}
