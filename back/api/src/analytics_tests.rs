#[cfg(test)]
mod analytics_tests {
    use chrono::Utc;
    use crate::analytics::usecase::AnalyticsUseCase;
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
    fn test_platform_stats_dto_serializes_with_non_finite_averages() {
        let mut stats = PlatformStatsDto {
            total_players: 1,
            total_contests: 0,
            total_games: 0,
            total_venues: 0,
            active_players_30d: 0,
            active_players_7d: 0,
            contests_30d: 0,
            average_participants_per_contest: f64::NAN,
            top_games: vec![GamePopularityDto {
                game_id: "game/1".into(),
                game_name: "Test".into(),
                plays: 1,
                popularity_score: f64::INFINITY,
            }],
            top_venues: vec![VenueActivityDto {
                venue_id: "venue/1".into(),
                venue_name: "Venue".into(),
                contests_held: 1,
                total_participants: 1,
                activity_score: f64::NEG_INFINITY,
            }],
            last_updated: Utc::now().fixed_offset(),
        };
        AnalyticsUseCase::sanitize_platform_stats_dto(&mut stats);
        let json = serde_json::to_string(&stats).expect("platform stats must serialize");
        assert!(json.contains("\"average_participants_per_contest\":0"));
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

    #[test]
    fn test_analytics_basic_operations() {
        // Test basic analytics operations without complex dependencies
        let data: Vec<i32> = vec![1, 2, 3, 4, 5];

        let sum: i32 = data.iter().sum();
        let avg = sum as f64 / data.len() as f64;

        assert_eq!(sum, 15);
        assert_eq!(avg, 3.0);
    }

    #[test]
    fn test_overview_tab_dto_shape() {
        let dto = OverviewTabDto {
            timezone: "America/Chicago".to_string(),
            new_players_30d: 1,
            returning_players_30d: 2,
            contest_completion_rate_pct: 80.0,
            week_over_week: WeekOverWeekDto {
                contests_this_week: 3,
                contests_last_week: 2,
                contests_change_pct: 50.0,
                players_this_week: 4,
                players_last_week: 4,
                players_change_pct: 0.0,
                weekly_contest_sparkline: vec![CountBucketDto {
                    label: "2026-W21".to_string(),
                    count: 1,
                }],
            },
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["timezone"], "America/Chicago");
        assert!(json.get("week_over_week").is_some());
    }

    #[test]
    fn test_contests_tab_dto_shape() {
        let dto = ContestsTabDto {
            timezone: "UTC".to_string(),
            avg_duration_minutes: 90.0,
            avg_time_to_fill_hours: 12.0,
            size_distribution: vec![],
            peak_participants_heatmap: vec![],
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["timezone"], "UTC");
        assert!(json.get("avg_duration_minutes").is_some());
    }

    #[test]
    fn test_contest_stats_dto_includes_game_id() {
        let dto = ContestStatsDto {
            contest_id: "contest/1".to_string(),
            contest_name: "Friday Night".to_string(),
            participant_count: 4,
            completion_count: 4,
            completion_rate: 100.0,
            average_placement: 2.5,
            duration_minutes: 120,
            most_popular_game: Some("Catan".to_string()),
            most_popular_game_id: Some("game/catan".to_string()),
            difficulty_rating: 5.0,
            excitement_rating: 6.0,
            started_at: None,
            last_updated: Utc::now().fixed_offset(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["most_popular_game_id"], "game/catan");
    }
}
