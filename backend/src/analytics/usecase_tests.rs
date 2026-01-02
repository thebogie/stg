#[cfg(test)]
mod analytics_usecase_tests {
    use super::*;
    use crate::analytics::{AnalyticsUseCase, AnalyticsRepository, AnalyticsEngine, AnalyticsCache};
    use shared::dto::analytics::*;
    use shared::models::analytics::*;
    use chrono::{Utc, Duration};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock repository for testing
    #[derive(Clone)]
    struct MockAnalyticsRepository {
        platform_stats: Arc<Mutex<PlatformStatsDto>>,
        leaderboard_entries: Arc<Mutex<Vec<LeaderboardEntryDto>>>,
        player_achievements: Arc<Mutex<PlayerAchievementsDto>>,
        monthly_contests: Arc<Mutex<Vec<MonthlyContestsDto>>>,
    }

    impl MockAnalyticsRepository {
        fn new() -> Self {
            Self {
                platform_stats: Arc::new(Mutex::new(PlatformStatsDto {
                    total_players: 100,
                    total_contests: 50,
                    total_games: 25,
                    total_venues: 10,
                    active_players_30d: 75,
                    active_players_7d: 25,
                    contests_30d: 15,
                    average_participants_per_contest: 4.0,
                    top_games: vec![],
                    top_venues: vec![],
                    last_updated: Utc::now().fixed_offset(),
                })),
                leaderboard_entries: Arc::new(Mutex::new(vec![])),
                player_achievements: Arc::new(Mutex::new(PlayerAchievementsDto {
                    player_id: "player/test".to_string(),
                    player_handle: "testuser".to_string(),
                    achievements: vec![],
                    total_achievements: 10,
                    unlocked_achievements: 5,
                    completion_percentage: 50.0,
                })),
                monthly_contests: Arc::new(Mutex::new(vec![])),
            }
        }

        fn set_platform_stats(&self, stats: PlatformStatsDto) {
            let mut platform_stats = self.platform_stats.blocking_lock();
            *platform_stats = stats;
        }

        fn set_leaderboard_entries(&self, entries: Vec<LeaderboardEntryDto>) {
            let mut leaderboard_entries = self.leaderboard_entries.blocking_lock();
            *leaderboard_entries = entries;
        }

        fn set_player_achievements(&self, achievements: PlayerAchievementsDto) {
            let mut player_achievements = self.player_achievements.blocking_lock();
            *player_achievements = achievements;
        }

        fn set_monthly_contests(&self, contests: Vec<MonthlyContestsDto>) {
            let mut monthly_contests = self.monthly_contests.blocking_lock();
            *monthly_contests = contests;
        }
    }

    #[async_trait::async_trait]
    impl AnalyticsRepository<arangors::client::reqwest::ReqwestClient> for MockAnalyticsRepository {
        async fn get_platform_stats(&self) -> Result<PlatformStatsDto> {
            let stats = self.platform_stats.lock().await;
            Ok(stats.clone())
        }

        async fn get_leaderboard(&self, category: LeaderboardCategory, time_period: TimePeriod) -> Result<Vec<LeaderboardEntryDto>> {
            let entries = self.leaderboard_entries.lock().await;
            Ok(entries.clone())
        }

        async fn get_player_achievements(&self, player_id: &str) -> Result<PlayerAchievementsDto> {
            let achievements = self.player_achievements.lock().await;
            Ok(achievements.clone())
        }

        async fn get_monthly_contests(&self, year: i32) -> Result<Vec<MonthlyContestsDto>> {
            let contests = self.monthly_contests.lock().await;
            Ok(contests.clone())
        }

        async fn get_player_statistics(&self, player_id: &str, time_period: TimePeriod) -> Result<PlayerStatisticsDto> {
            Ok(PlayerStatisticsDto {
                player_id: player_id.to_string(),
                player_handle: "testuser".to_string(),
                time_period,
                total_games: 10,
                wins: 7,
                losses: 3,
                win_rate: 0.7,
                average_placement: 2.1,
                total_rating_change: Some(45.0),
                current_rating: 1245.0,
                best_rating: 1300.0,
                worst_rating: 1200.0,
                last_updated: Utc::now().fixed_offset(),
            })
        }

        async fn get_game_statistics(&self, game_id: &str, time_period: TimePeriod) -> Result<GameStatisticsDto> {
            Ok(GameStatisticsDto {
                game_id: game_id.to_string(),
                game_name: "Test Game".to_string(),
                time_period,
                total_contests: 5,
                total_players: 20,
                average_participants: 4.0,
                most_common_placement: 2,
                last_played: Some(Utc::now().fixed_offset()),
                last_updated: Utc::now().fixed_offset(),
            })
        }

        async fn get_venue_statistics(&self, venue_id: &str, time_period: TimePeriod) -> Result<VenueStatisticsDto> {
            Ok(VenueStatisticsDto {
                venue_id: venue_id.to_string(),
                venue_name: "Test Venue".to_string(),
                time_period,
                total_contests: 8,
                total_players: 32,
                average_participants: 4.0,
                most_popular_game: Some("Test Game".to_string()),
                last_contest: Some(Utc::now().fixed_offset()),
                last_updated: Utc::now().fixed_offset(),
            })
        }
    }

    #[tokio::test]
    async fn test_analytics_usecase_creation() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        assert!(usecase.repo().get_platform_stats().await.is_ok());
    }

    #[tokio::test]
    async fn test_analytics_usecase_with_cache() {
        let repo = MockAnalyticsRepository::new();
        let cache = AnalyticsCache::new_default();
        let usecase = AnalyticsUseCase::with_cache(repo, cache);
        
        assert!(usecase.repo().get_platform_stats().await.is_ok());
    }

    #[tokio::test]
    async fn test_get_platform_stats() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        let stats = usecase.get_platform_stats().await.unwrap();
        
        assert_eq!(stats.total_players, 100);
        assert_eq!(stats.total_contests, 50);
        assert_eq!(stats.total_games, 25);
        assert_eq!(stats.total_venues, 10);
        assert_eq!(stats.active_players_30d, 75);
        assert_eq!(stats.active_players_7d, 25);
        assert_eq!(stats.contests_30d, 15);
        assert_eq!(stats.average_participants_per_contest, 4.0);
    }

    #[tokio::test]
    async fn test_get_platform_stats_cached() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        // First call should hit repository
        let stats1 = usecase.get_platform_stats().await.unwrap();
        
        // Second call should hit cache
        let stats2 = usecase.get_platform_stats().await.unwrap();
        
        assert_eq!(stats1.total_players, stats2.total_players);
        assert_eq!(stats1.total_contests, stats2.total_contests);
    }

    #[tokio::test]
    async fn test_get_leaderboard() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        let leaderboard = usecase.get_leaderboard(LeaderboardCategory::WinRate, TimePeriod::Last30Days).await.unwrap();
        
        assert!(leaderboard.is_empty()); // Mock returns empty by default
    }

    #[tokio::test]
    async fn test_get_leaderboard_with_data() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        let entries = vec![
            LeaderboardEntryDto {
                player_id: "player/1".to_string(),
                player_handle: "player1".to_string(),
                score: 0.8,
                rank: 1,
                games_played: 10,
                wins: 8,
                losses: 2,
            },
            LeaderboardEntryDto {
                player_id: "player/2".to_string(),
                player_handle: "player2".to_string(),
                score: 0.6,
                rank: 2,
                games_played: 10,
                wins: 6,
                losses: 4,
            },
        ];
        
        repo.set_leaderboard_entries(entries.clone());
        
        let leaderboard = usecase.get_leaderboard(LeaderboardCategory::WinRate, TimePeriod::Last30Days).await.unwrap();
        
        assert_eq!(leaderboard.len(), 2);
        assert_eq!(leaderboard[0].player_handle, "player1");
        assert_eq!(leaderboard[1].player_handle, "player2");
    }

    #[tokio::test]
    async fn test_get_player_achievements() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        let achievements = usecase.get_player_achievements("player/test").await.unwrap();
        
        assert_eq!(achievements.player_id, "player/test");
        assert_eq!(achievements.player_handle, "testuser");
        assert_eq!(achievements.total_achievements, 10);
        assert_eq!(achievements.unlocked_achievements, 5);
        assert_eq!(achievements.completion_percentage, 50.0);
    }

    #[tokio::test]
    async fn test_get_monthly_contests() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        let contests = usecase.get_monthly_contests(2024).await.unwrap();
        
        assert!(contests.is_empty()); // Mock returns empty by default
    }

    #[tokio::test]
    async fn test_get_monthly_contests_with_data() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        let monthly_contests = vec![
            MonthlyContestsDto {
                year: 2024,
                month: 1,
                contests: 5,
            },
            MonthlyContestsDto {
                year: 2024,
                month: 2,
                contests: 8,
            },
        ];
        
        repo.set_monthly_contests(monthly_contests.clone());
        
        let contests = usecase.get_monthly_contests(2024).await.unwrap();
        
        assert_eq!(contests.len(), 2);
        assert_eq!(contests[0].month, 1);
        assert_eq!(contests[0].contests, 5);
        assert_eq!(contests[1].month, 2);
        assert_eq!(contests[1].contests, 8);
    }

    #[tokio::test]
    async fn test_get_player_statistics() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        let stats = usecase.get_player_statistics("player/test", TimePeriod::Last30Days).await.unwrap();
        
        assert_eq!(stats.player_id, "player/test");
        assert_eq!(stats.player_handle, "testuser");
        assert_eq!(stats.time_period, TimePeriod::Last30Days);
        assert_eq!(stats.total_games, 10);
        assert_eq!(stats.wins, 7);
        assert_eq!(stats.losses, 3);
        assert_eq!(stats.win_rate, 0.7);
        assert_eq!(stats.average_placement, 2.1);
        assert_eq!(stats.total_rating_change, Some(45.0));
        assert_eq!(stats.current_rating, 1245.0);
        assert_eq!(stats.best_rating, 1300.0);
        assert_eq!(stats.worst_rating, 1200.0);
    }

    #[tokio::test]
    async fn test_get_game_statistics() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        let stats = usecase.get_game_statistics("game/test", TimePeriod::Last30Days).await.unwrap();
        
        assert_eq!(stats.game_id, "game/test");
        assert_eq!(stats.game_name, "Test Game");
        assert_eq!(stats.time_period, TimePeriod::Last30Days);
        assert_eq!(stats.total_contests, 5);
        assert_eq!(stats.total_players, 20);
        assert_eq!(stats.average_participants, 4.0);
        assert_eq!(stats.most_common_placement, 2);
        assert!(stats.last_played.is_some());
    }

    #[tokio::test]
    async fn test_get_venue_statistics() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        let stats = usecase.get_venue_statistics("venue/test", TimePeriod::Last30Days).await.unwrap();
        
        assert_eq!(stats.venue_id, "venue/test");
        assert_eq!(stats.venue_name, "Test Venue");
        assert_eq!(stats.time_period, TimePeriod::Last30Days);
        assert_eq!(stats.total_contests, 8);
        assert_eq!(stats.total_players, 32);
        assert_eq!(stats.average_participants, 4.0);
        assert_eq!(stats.most_popular_game, Some("Test Game".to_string()));
        assert!(stats.last_contest.is_some());
    }

    #[tokio::test]
    async fn test_get_leaderboard_different_categories() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        // Test WinRate category
        let winrate_leaderboard = usecase.get_leaderboard(LeaderboardCategory::WinRate, TimePeriod::Last30Days).await.unwrap();
        assert!(winrate_leaderboard.is_empty());
        
        // Test TotalWins category
        let totalwins_leaderboard = usecase.get_leaderboard(LeaderboardCategory::TotalWins, TimePeriod::Last30Days).await.unwrap();
        assert!(totalwins_leaderboard.is_empty());
        
        // Test SkillRating category
        let skillrating_leaderboard = usecase.get_leaderboard(LeaderboardCategory::SkillRating, TimePeriod::Last30Days).await.unwrap();
        assert!(skillrating_leaderboard.is_empty());
    }

    #[tokio::test]
    async fn test_get_leaderboard_different_time_periods() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        // Test Last7Days
        let last7days = usecase.get_leaderboard(LeaderboardCategory::WinRate, TimePeriod::Last7Days).await.unwrap();
        assert!(last7days.is_empty());
        
        // Test Last30Days
        let last30days = usecase.get_leaderboard(LeaderboardCategory::WinRate, TimePeriod::Last30Days).await.unwrap();
        assert!(last30days.is_empty());
        
        // Test AllTime
        let alltime = usecase.get_leaderboard(LeaderboardCategory::WinRate, TimePeriod::AllTime).await.unwrap();
        assert!(alltime.is_empty());
    }

    #[tokio::test]
    async fn test_get_player_statistics_different_time_periods() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        // Test Last7Days
        let last7days = usecase.get_player_statistics("player/test", TimePeriod::Last7Days).await.unwrap();
        assert_eq!(last7days.time_period, TimePeriod::Last7Days);
        
        // Test Last30Days
        let last30days = usecase.get_player_statistics("player/test", TimePeriod::Last30Days).await.unwrap();
        assert_eq!(last30days.time_period, TimePeriod::Last30Days);
        
        // Test AllTime
        let alltime = usecase.get_player_statistics("player/test", TimePeriod::AllTime).await.unwrap();
        assert_eq!(alltime.time_period, TimePeriod::AllTime);
    }

    #[tokio::test]
    async fn test_get_game_statistics_different_time_periods() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        // Test Last7Days
        let last7days = usecase.get_game_statistics("game/test", TimePeriod::Last7Days).await.unwrap();
        assert_eq!(last7days.time_period, TimePeriod::Last7Days);
        
        // Test Last30Days
        let last30days = usecase.get_game_statistics("game/test", TimePeriod::Last30Days).await.unwrap();
        assert_eq!(last30days.time_period, TimePeriod::Last30Days);
        
        // Test AllTime
        let alltime = usecase.get_game_statistics("game/test", TimePeriod::AllTime).await.unwrap();
        assert_eq!(alltime.time_period, TimePeriod::AllTime);
    }

    #[tokio::test]
    async fn test_get_venue_statistics_different_time_periods() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        // Test Last7Days
        let last7days = usecase.get_venue_statistics("venue/test", TimePeriod::Last7Days).await.unwrap();
        assert_eq!(last7days.time_period, TimePeriod::Last7Days);
        
        // Test Last30Days
        let last30days = usecase.get_venue_statistics("venue/test", TimePeriod::Last30Days).await.unwrap();
        assert_eq!(last30days.time_period, TimePeriod::Last30Days);
        
        // Test AllTime
        let alltime = usecase.get_venue_statistics("venue/test", TimePeriod::AllTime).await.unwrap();
        assert_eq!(alltime.time_period, TimePeriod::AllTime);
    }

    #[tokio::test]
    async fn test_usecase_clone() {
        let repo = MockAnalyticsRepository::new();
        let usecase1 = AnalyticsUseCase::new(repo);
        let usecase2 = usecase1.clone();
        
        // Both should be able to access the same repository
        let stats1 = usecase1.get_platform_stats().await.unwrap();
        let stats2 = usecase2.get_platform_stats().await.unwrap();
        
        assert_eq!(stats1.total_players, stats2.total_players);
        assert_eq!(stats1.total_contests, stats2.total_contests);
    }

    #[tokio::test]
    async fn test_error_handling() {
        // Test with a repository that returns errors
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        // All operations should succeed with mock repository
        let stats = usecase.get_platform_stats().await;
        assert!(stats.is_ok());
        
        let leaderboard = usecase.get_leaderboard(LeaderboardCategory::WinRate, TimePeriod::Last30Days).await;
        assert!(leaderboard.is_ok());
        
        let achievements = usecase.get_player_achievements("player/test").await;
        assert!(achievements.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let repo = MockAnalyticsRepository::new();
        let usecase = AnalyticsUseCase::new(repo);
        
        // Test concurrent access to different methods
        let (stats_result, leaderboard_result, achievements_result) = tokio::join!(
            usecase.get_platform_stats(),
            usecase.get_leaderboard(LeaderboardCategory::WinRate, TimePeriod::Last30Days),
            usecase.get_player_achievements("player/test")
        );
        
        assert!(stats_result.is_ok());
        assert!(leaderboard_result.is_ok());
        assert!(achievements_result.is_ok());
    }
}
