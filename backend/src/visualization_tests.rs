#[cfg(test)]
mod visualization_tests {
    use chrono::Utc;
    use shared::dto::analytics::*;

    #[test]
    fn test_leaderboard_response_creation() {
        let response = LeaderboardResponse {
            category: LeaderboardCategory::WinRate,
            time_period: TimePeriod::Last30Days,
            entries: vec![],
            total_entries: 0,
            last_updated: Utc::now().fixed_offset(),
        };

        assert!(matches!(response.category, LeaderboardCategory::WinRate));
        assert!(matches!(response.time_period, TimePeriod::Last30Days));
        assert_eq!(response.total_entries, 0);
    }

    #[test]
    fn test_player_achievements_dto() {
        let achievements = PlayerAchievementsDto {
            player_id: "player/test".to_string(),
            player_handle: "testuser".to_string(),
            achievements: vec![],
            total_achievements: 10,
            unlocked_achievements: 5,
            completion_percentage: 50.0,
        };

        assert_eq!(achievements.player_id, "player/test");
        assert_eq!(achievements.player_handle, "testuser");
        assert_eq!(achievements.total_achievements, 10);
        assert_eq!(achievements.unlocked_achievements, 5);
        assert_eq!(achievements.completion_percentage, 50.0);
    }

    #[test]
    fn test_monthly_contests_dto() {
        let contests = MonthlyContestsDto {
            year: 2024,
            month: 1,
            contests: 5,
        };

        assert_eq!(contests.year, 2024);
        assert_eq!(contests.month, 1);
        assert_eq!(contests.contests, 5);
    }

    #[test]
    fn test_platform_stats_dto() {
        let stats = PlatformStatsDto {
            total_players: 200,
            total_contests: 50,
            total_games: 100,
            total_venues: 10,
            active_players_30d: 150,
            active_players_7d: 75,
            contests_30d: 25,
            average_participants_per_contest: 4.0,
            top_games: vec![],
            top_venues: vec![],
            last_updated: Utc::now().fixed_offset(),
        };

        assert_eq!(stats.total_games, 100);
        assert_eq!(stats.total_venues, 10);
        assert_eq!(stats.total_contests, 50);
        assert_eq!(stats.total_players, 200);
    }

    #[test]
    fn test_leaderboard_category_enum() {
        // Test that LeaderboardCategory enum variants exist
        match LeaderboardCategory::WinRate {
            LeaderboardCategory::WinRate => assert!(true),
            _ => assert!(false),
        }

        match LeaderboardCategory::TotalWins {
            LeaderboardCategory::TotalWins => assert!(true),
            _ => assert!(false),
        }

        match LeaderboardCategory::SkillRating {
            LeaderboardCategory::SkillRating => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_time_period_enum() {
        // Test that TimePeriod enum variants exist
        match TimePeriod::AllTime {
            TimePeriod::AllTime => assert!(true),
            _ => assert!(false),
        }

        match TimePeriod::Last30Days {
            TimePeriod::Last30Days => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_dto_serialization() {
        let achievements = PlayerAchievementsDto {
            player_id: "player/test".to_string(),
            player_handle: "testuser".to_string(),
            achievements: vec![],
            total_achievements: 10,
            unlocked_achievements: 5,
            completion_percentage: 50.0,
        };

        let json = serde_json::to_string(&achievements).unwrap();
        let deserialized: PlayerAchievementsDto = serde_json::from_str(&json).unwrap();

        assert_eq!(achievements.player_id, deserialized.player_id);
        assert_eq!(achievements.player_handle, deserialized.player_handle);
        assert_eq!(
            achievements.total_achievements,
            deserialized.total_achievements
        );
    }
}
