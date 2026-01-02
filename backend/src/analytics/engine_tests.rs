#[cfg(test)]
mod analytics_engine_tests {
    use super::*;
    use crate::analytics::engine::{AnalyticsEngine, EloConfig};
    use shared::models::analytics::*;
    use shared::dto::analytics::*;
    use chrono::{Utc, Duration};

    #[test]
    fn test_analytics_engine_creation() {
        let engine = AnalyticsEngine::new();
        assert!(engine.elo_config.starting_rating > 0.0);
        assert!(engine.elo_config.k_factor > 0.0);
    }

    #[test]
    fn test_analytics_engine_with_custom_config() {
        let custom_config = EloConfig {
            starting_rating: 1500.0,
            k_factor: 40.0,
            rating_deviation: 300.0,
            min_rating_change: -100.0,
            max_rating_change: 100.0,
        };
        
        let engine = AnalyticsEngine::with_config(custom_config.clone());
        assert_eq!(engine.elo_config.starting_rating, 1500.0);
        assert_eq!(engine.elo_config.k_factor, 40.0);
        assert_eq!(engine.elo_config.rating_deviation, 300.0);
    }

    #[test]
    fn test_elo_config_default() {
        let config = EloConfig::default();
        assert_eq!(config.starting_rating, 1200.0);
        assert_eq!(config.k_factor, 32.0);
        assert_eq!(config.rating_deviation, 200.0);
        assert_eq!(config.min_rating_change, -50.0);
        assert_eq!(config.max_rating_change, 50.0);
    }

    #[test]
    fn test_calculate_win_rate() {
        let engine = AnalyticsEngine::new();
        
        // Test with various win/loss scenarios
        assert_eq!(engine.calculate_win_rate(10, 0), 1.0); // 100% win rate
        assert_eq!(engine.calculate_win_rate(0, 10), 0.0); // 0% win rate
        assert_eq!(engine.calculate_win_rate(5, 5), 0.5); // 50% win rate
        assert_eq!(engine.calculate_win_rate(3, 7), 0.3); // 30% win rate
    }

    #[test]
    fn test_calculate_win_rate_edge_cases() {
        let engine = AnalyticsEngine::new();
        
        // Test with zero games
        assert_eq!(engine.calculate_win_rate(0, 0), 0.0);
        
        // Test with very large numbers
        assert_eq!(engine.calculate_win_rate(1000, 1000), 0.5);
    }

    #[test]
    fn test_calculate_elo_rating_change() {
        let engine = AnalyticsEngine::new();
        
        // Test win against higher rated player
        let change = engine.calculate_elo_rating_change(1200.0, 1300.0, true);
        assert!(change > 0.0); // Should gain rating
        
        // Test loss against higher rated player
        let change = engine.calculate_elo_rating_change(1200.0, 1300.0, false);
        assert!(change < 0.0); // Should lose rating
        
        // Test win against lower rated player
        let change = engine.calculate_elo_rating_change(1300.0, 1200.0, true);
        assert!(change > 0.0); // Should gain rating (but less)
        
        // Test loss against lower rated player
        let change = engine.calculate_elo_rating_change(1300.0, 1200.0, false);
        assert!(change < 0.0); // Should lose rating (more)
    }

    #[test]
    fn test_calculate_elo_rating_change_bounds() {
        let engine = AnalyticsEngine::new();
        
        // Test that rating changes are within bounds
        let change = engine.calculate_elo_rating_change(1200.0, 1300.0, true);
        assert!(change >= engine.elo_config.min_rating_change);
        assert!(change <= engine.elo_config.max_rating_change);
        
        let change = engine.calculate_elo_rating_change(1200.0, 1300.0, false);
        assert!(change >= engine.elo_config.min_rating_change);
        assert!(change <= engine.elo_config.max_rating_change);
    }

    #[test]
    fn test_calculate_expected_score() {
        let engine = AnalyticsEngine::new();
        
        // Test equal ratings
        let expected = engine.calculate_expected_score(1200.0, 1200.0);
        assert!((expected - 0.5).abs() < 0.01); // Should be close to 0.5
        
        // Test higher rated player
        let expected = engine.calculate_expected_score(1300.0, 1200.0);
        assert!(expected > 0.5); // Should be > 0.5
        
        // Test lower rated player
        let expected = engine.calculate_expected_score(1200.0, 1300.0);
        assert!(expected < 0.5); // Should be < 0.5
    }

    #[test]
    fn test_calculate_achievement_progress() {
        let engine = AnalyticsEngine::new();
        
        // Test win streak achievement
        let progress = engine.calculate_achievement_progress(
            AchievementType::WinStreak,
            AchievementTarget::WinStreak(5),
            &AchievementProgress {
                current_value: 3,
                target_value: 5,
                is_completed: false,
            }
        );
        
        assert_eq!(progress.current_value, 3);
        assert_eq!(progress.target_value, 5);
        assert_eq!(progress.is_completed, false);
        assert_eq!(progress.progress_percentage, 60.0);
    }

    #[test]
    fn test_calculate_achievement_progress_completed() {
        let engine = AnalyticsEngine::new();
        
        // Test completed achievement
        let progress = engine.calculate_achievement_progress(
            AchievementType::WinStreak,
            AchievementTarget::WinStreak(5),
            &AchievementProgress {
                current_value: 5,
                target_value: 5,
                is_completed: true,
            }
        );
        
        assert_eq!(progress.current_value, 5);
        assert_eq!(progress.target_value, 5);
        assert_eq!(progress.is_completed, true);
        assert_eq!(progress.progress_percentage, 100.0);
    }

    #[test]
    fn test_calculate_leaderboard_score() {
        let engine = AnalyticsEngine::new();
        
        // Test win rate leaderboard
        let score = engine.calculate_leaderboard_score(
            LeaderboardCategory::WinRate,
            &LeaderboardEntry {
                player_id: "player/test".to_string(),
                player_handle: "testuser".to_string(),
                score: 0.75,
                rank: 1,
                games_played: 20,
                wins: 15,
                losses: 5,
            }
        );
        
        assert_eq!(score, 0.75);
    }

    #[test]
    fn test_calculate_leaderboard_score_total_wins() {
        let engine = AnalyticsEngine::new();
        
        // Test total wins leaderboard
        let score = engine.calculate_leaderboard_score(
            LeaderboardCategory::TotalWins,
            &LeaderboardEntry {
                player_id: "player/test".to_string(),
                player_handle: "testuser".to_string(),
                score: 15.0,
                rank: 1,
                games_played: 20,
                wins: 15,
                losses: 5,
            }
        );
        
        assert_eq!(score, 15.0);
    }

    #[test]
    fn test_calculate_leaderboard_score_skill_rating() {
        let engine = AnalyticsEngine::new();
        
        // Test skill rating leaderboard
        let score = engine.calculate_leaderboard_score(
            LeaderboardCategory::SkillRating,
            &LeaderboardEntry {
                player_id: "player/test".to_string(),
                player_handle: "testuser".to_string(),
                score: 1350.0,
                rank: 1,
                games_played: 20,
                wins: 15,
                losses: 5,
            }
        );
        
        assert_eq!(score, 1350.0);
    }

    #[test]
    fn test_calculate_statistics() {
        let engine = AnalyticsEngine::new();
        
        let stats = engine.calculate_statistics(&vec![
            ContestResult {
                contest_id: "contest/1".to_string(),
                player_id: "player/test".to_string(),
                placement: 1,
                total_players: 4,
                rating_change: Some(25.0),
                timestamp: Utc::now().fixed_offset(),
            },
            ContestResult {
                contest_id: "contest/2".to_string(),
                player_id: "player/test".to_string(),
                placement: 2,
                total_players: 4,
                rating_change: Some(-10.0),
                timestamp: Utc::now().fixed_offset(),
            },
        ]);
        
        assert_eq!(stats.total_games, 2);
        assert_eq!(stats.wins, 1);
        assert_eq!(stats.losses, 1);
        assert_eq!(stats.win_rate, 0.5);
        assert_eq!(stats.average_placement, 1.5);
        assert_eq!(stats.total_rating_change, Some(15.0));
    }

    #[test]
    fn test_calculate_statistics_empty() {
        let engine = AnalyticsEngine::new();
        
        let stats = engine.calculate_statistics(&vec![]);
        
        assert_eq!(stats.total_games, 0);
        assert_eq!(stats.wins, 0);
        assert_eq!(stats.losses, 0);
        assert_eq!(stats.win_rate, 0.0);
        assert_eq!(stats.average_placement, 0.0);
        assert_eq!(stats.total_rating_change, None);
    }

    #[test]
    fn test_calculate_statistics_with_rating_changes() {
        let engine = AnalyticsEngine::new();
        
        let stats = engine.calculate_statistics(&vec![
            ContestResult {
                contest_id: "contest/1".to_string(),
                player_id: "player/test".to_string(),
                placement: 1,
                total_players: 4,
                rating_change: Some(25.0),
                timestamp: Utc::now().fixed_offset(),
            },
            ContestResult {
                contest_id: "contest/2".to_string(),
                player_id: "player/test".to_string(),
                placement: 2,
                total_players: 4,
                rating_change: Some(-10.0),
                timestamp: Utc::now().fixed_offset(),
            },
            ContestResult {
                contest_id: "contest/3".to_string(),
                player_id: "player/test".to_string(),
                placement: 1,
                total_players: 4,
                rating_change: None, // No rating change
                timestamp: Utc::now().fixed_offset(),
            },
        ]);
        
        assert_eq!(stats.total_games, 3);
        assert_eq!(stats.wins, 2);
        assert_eq!(stats.losses, 1);
        assert_eq!(stats.win_rate, 2.0 / 3.0);
        assert_eq!(stats.average_placement, 4.0 / 3.0);
        assert_eq!(stats.total_rating_change, Some(15.0)); // Only sum non-None values
    }

    #[test]
    fn test_calculate_period_statistics() {
        let engine = AnalyticsEngine::new();
        
        let now = Utc::now().fixed_offset();
        let week_ago = now - Duration::days(7);
        let month_ago = now - Duration::days(30);
        
        let stats = engine.calculate_period_statistics(&vec![
            ContestResult {
                contest_id: "contest/1".to_string(),
                player_id: "player/test".to_string(),
                placement: 1,
                total_players: 4,
                rating_change: Some(25.0),
                timestamp: week_ago,
            },
            ContestResult {
                contest_id: "contest/2".to_string(),
                player_id: "player/test".to_string(),
                placement: 2,
                total_players: 4,
                rating_change: Some(-10.0),
                timestamp: month_ago,
            },
        ], TimePeriod::Last30Days);
        
        assert_eq!(stats.total_games, 2);
        assert_eq!(stats.wins, 1);
        assert_eq!(stats.losses, 1);
        assert_eq!(stats.win_rate, 0.5);
    }

    #[test]
    fn test_calculate_period_statistics_last_7_days() {
        let engine = AnalyticsEngine::new();
        
        let now = Utc::now().fixed_offset();
        let week_ago = now - Duration::days(7);
        let month_ago = now - Duration::days(30);
        
        let stats = engine.calculate_period_statistics(&vec![
            ContestResult {
                contest_id: "contest/1".to_string(),
                player_id: "player/test".to_string(),
                placement: 1,
                total_players: 4,
                rating_change: Some(25.0),
                timestamp: week_ago,
            },
            ContestResult {
                contest_id: "contest/2".to_string(),
                player_id: "player/test".to_string(),
                placement: 2,
                total_players: 4,
                rating_change: Some(-10.0),
                timestamp: month_ago,
            },
        ], TimePeriod::Last7Days);
        
        assert_eq!(stats.total_games, 1); // Only the week_ago game
        assert_eq!(stats.wins, 1);
        assert_eq!(stats.losses, 0);
        assert_eq!(stats.win_rate, 1.0);
    }

    #[test]
    fn test_calculate_period_statistics_all_time() {
        let engine = AnalyticsEngine::new();
        
        let now = Utc::now().fixed_offset();
        let week_ago = now - Duration::days(7);
        let month_ago = now - Duration::days(30);
        let year_ago = now - Duration::days(365);
        
        let stats = engine.calculate_period_statistics(&vec![
            ContestResult {
                contest_id: "contest/1".to_string(),
                player_id: "player/test".to_string(),
                placement: 1,
                total_players: 4,
                rating_change: Some(25.0),
                timestamp: week_ago,
            },
            ContestResult {
                contest_id: "contest/2".to_string(),
                player_id: "player/test".to_string(),
                placement: 2,
                total_players: 4,
                rating_change: Some(-10.0),
                timestamp: month_ago,
            },
            ContestResult {
                contest_id: "contest/3".to_string(),
                player_id: "player/test".to_string(),
                placement: 1,
                total_players: 4,
                rating_change: Some(30.0),
                timestamp: year_ago,
            },
        ], TimePeriod::AllTime);
        
        assert_eq!(stats.total_games, 3);
        assert_eq!(stats.wins, 2);
        assert_eq!(stats.losses, 1);
        assert_eq!(stats.win_rate, 2.0 / 3.0);
    }

    #[test]
    fn test_elo_config_validation() {
        let config = EloConfig {
            starting_rating: 1200.0,
            k_factor: 32.0,
            rating_deviation: 200.0,
            min_rating_change: -50.0,
            max_rating_change: 50.0,
        };
        
        // Test that config values are reasonable
        assert!(config.starting_rating > 0.0);
        assert!(config.k_factor > 0.0);
        assert!(config.rating_deviation > 0.0);
        assert!(config.min_rating_change < 0.0);
        assert!(config.max_rating_change > 0.0);
        assert!(config.max_rating_change > config.min_rating_change.abs());
    }

    #[test]
    fn test_analytics_engine_clone() {
        let engine1 = AnalyticsEngine::new();
        let engine2 = engine1.clone();
        
        assert_eq!(engine1.elo_config.starting_rating, engine2.elo_config.starting_rating);
        assert_eq!(engine1.elo_config.k_factor, engine2.elo_config.k_factor);
    }

    #[test]
    fn test_elo_config_debug() {
        let config = EloConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("starting_rating"));
        assert!(debug_str.contains("k_factor"));
        assert!(debug_str.contains("rating_deviation"));
    }

    #[test]
    fn test_elo_config_clone() {
        let config1 = EloConfig::default();
        let config2 = config1.clone();
        
        assert_eq!(config1.starting_rating, config2.starting_rating);
        assert_eq!(config1.k_factor, config2.k_factor);
        assert_eq!(config1.rating_deviation, config2.rating_deviation);
        assert_eq!(config1.min_rating_change, config2.min_rating_change);
        assert_eq!(config1.max_rating_change, config2.max_rating_change);
    }
}
