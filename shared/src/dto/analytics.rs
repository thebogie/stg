use crate::models::analytics::*;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Data Transfer Object for Player Statistics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlayerStatsDto {
    pub player_id: String,
    pub player_handle: String,
    pub player_name: String,
    pub total_contests: i32,
    pub total_wins: i32,
    pub total_losses: i32,
    pub win_rate: f64,
    pub average_placement: f64,
    pub best_placement: i32,
    pub skill_rating: f64,
    pub rating_confidence: f64,
    pub total_points: i32,
    pub current_streak: i32,
    pub longest_streak: i32,
    pub last_updated: DateTime<FixedOffset>,
}

/// Data Transfer Object for Contest Statistics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ContestStatsDto {
    pub contest_id: String,
    pub contest_name: String,
    pub participant_count: i32,
    pub completion_count: i32,
    pub completion_rate: f64,
    pub average_placement: f64,
    pub duration_minutes: i32,
    pub most_popular_game: Option<String>,
    pub difficulty_rating: f64,
    pub excitement_rating: f64,
    pub last_updated: DateTime<FixedOffset>,
}

/// Data Transfer Object for Game Statistics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GameStatsDto {
    pub game_id: String,
    pub game_name: String,
    pub total_plays: i32,
    pub unique_players: i32,
    pub average_players: f64,
    pub win_rate_distribution: Vec<PlayerWinRateDto>,
    pub popularity_trend: Vec<MonthlyPlaysDto>,
    pub average_duration_minutes: f64,
    pub last_updated: DateTime<FixedOffset>,
}

/// Data Transfer Object for Venue Statistics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct VenueStatsDto {
    pub venue_id: String,
    pub venue_name: String,
    pub total_contests: i32,
    pub unique_players: i32,
    pub average_participants: f64,
    pub popular_games: Vec<GamePopularityDto>,
    pub monthly_contests: Vec<MonthlyContestsDto>,
    pub average_duration_minutes: f64,
    pub last_updated: DateTime<FixedOffset>,
}

/// Data Transfer Object for Platform Statistics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlatformStatsDto {
    pub total_players: i32,
    pub total_contests: i32,
    pub total_games: i32,
    pub total_venues: i32,
    pub active_players_30d: i32,
    pub active_players_7d: i32,
    pub contests_30d: i32,
    pub average_participants_per_contest: f64,
    pub top_games: Vec<GamePopularityDto>,
    pub top_venues: Vec<VenueActivityDto>,
    pub last_updated: DateTime<FixedOffset>,
}

/// Data Transfer Object for Player Win Rate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerWinRateDto {
    pub player_id: String,
    pub player_handle: String,
    pub wins: i32,
    pub total_plays: i32,
    pub win_rate: f64,
}

/// Data Transfer Object for Monthly Plays
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyPlaysDto {
    pub year: i32,
    pub month: u32,
    pub plays: i32,
}

/// Data Transfer Object for Game Popularity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePopularityDto {
    pub game_id: String,
    pub game_name: String,
    pub plays: i32,
    pub popularity_score: f64,
}

/// Data Transfer Object for Monthly Contests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyContestsDto {
    pub year: i32,
    pub month: u32,
    pub contests: i32,
}

/// Data Transfer Object for Venue Activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueActivityDto {
    pub venue_id: String,
    pub venue_name: String,
    pub contests_held: i32,
    pub total_participants: i32,
    pub activity_score: f64,
}

/// Data Transfer Object for Achievement
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AchievementDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: AchievementCategoryDto,
    pub required_value: i32,
    pub current_value: i32,
    pub unlocked: bool,
    pub unlocked_at: Option<DateTime<FixedOffset>>,
}

/// Data Transfer Object for Achievement Category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AchievementCategoryDto {
    #[serde(rename = "wins")]
    Wins,
    #[serde(rename = "contests")]
    Contests,
    #[serde(rename = "streaks")]
    Streaks,
    #[serde(rename = "games")]
    Games,
    #[serde(rename = "venues")]
    Venues,
    #[serde(rename = "special")]
    Special,
}

impl std::fmt::Display for AchievementCategoryDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AchievementCategoryDto::Wins => write!(f, "Wins"),
            AchievementCategoryDto::Contests => write!(f, "Contests"),
            AchievementCategoryDto::Streaks => write!(f, "Streaks"),
            AchievementCategoryDto::Games => write!(f, "Games"),
            AchievementCategoryDto::Venues => write!(f, "Venues"),
            AchievementCategoryDto::Special => write!(f, "Special"),
        }
    }
}

/// Data Transfer Object for Player Achievements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerAchievementsDto {
    pub player_id: String,
    pub player_handle: String,
    pub achievements: Vec<AchievementDto>,
    pub total_achievements: i32,
    pub unlocked_achievements: i32,
    pub completion_percentage: f64,
}

/// Data Transfer Object for Player Ranking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRankingDto {
    pub category: String,
    pub rank: i32,
    pub total_players: i32,
    pub value: f64,
}

/// Data Transfer Object for Player Data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerDataDto {
    pub player_id: String,
    pub player_handle: String,
    pub total_contests: i32,
    pub total_wins: i32,
    pub unique_games: i32,
    pub unique_venues: i32,
}

/// Data Transfer Object for Player Opponent (players you've faced)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerOpponentDto {
    pub player_id: String,
    pub player_handle: String,
    pub player_name: String,
    pub contests_played: i32,
    pub wins_against_me: i32,
    pub losses_to_me: i32,
    pub win_rate_against_me: f64,
    pub last_played: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub total_contests: i32,
    pub overall_win_rate: f64,
}

/// Data Transfer Object for Game Performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePerformanceDto {
    pub game_id: String,
    pub game_name: String,
    pub total_plays: i32,
    pub wins: i32,
    pub losses: i32,
    pub win_rate: f64,
    pub average_placement: f64,
    pub best_placement: i32,
    pub worst_placement: i32,
    pub total_points: i32,
    pub average_points: f64,
    pub last_played: chrono::DateTime<chrono::FixedOffset>,
    pub days_since_last_play: i64,
    pub favorite_venue: Option<String>,
}

/// Data Transfer Object for Head-to-Head Record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeadToHeadRecordDto {
    pub opponent_id: String,
    pub opponent_handle: String,
    pub opponent_name: String,
    pub total_contests: i32,
    pub my_wins: i32,
    pub opponent_wins: i32,
    pub my_win_rate: f64,
    pub contest_history: Vec<HeadToHeadContestDto>,
}

/// Data Transfer Object for Head-to-Head Contest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeadToHeadContestDto {
    pub contest_id: String,
    pub contest_name: String,
    pub game_id: Option<String>,
    pub game_name: String,
    pub venue_id: Option<String>,
    pub venue_name: String,
    pub my_placement: i32,
    pub opponent_placement: i32,
    pub i_won: bool,
    pub contest_date: chrono::DateTime<chrono::FixedOffset>,
}

/// Data Transfer Object for Performance Trends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrendDto {
    pub month: String, // Format: "2024-01"
    pub contests_played: i32,
    pub wins: i32,
    pub win_rate: f64,
    pub average_placement: f64,
    pub skill_rating: f64,
    pub points_earned: i32,
}

/// Request for player statistics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlayerStatsRequest {
    pub player_id: String,
    pub include_achievements: bool,
    pub include_trends: bool,
}

/// Request for contest statistics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ContestStatsRequest {
    pub contest_id: String,
    pub include_participants: bool,
    pub include_games: bool,
}

/// Request for game statistics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GameStatsRequest {
    pub game_id: String,
    pub include_trends: bool,
    pub include_players: bool,
}

/// Request for venue statistics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct VenueStatsRequest {
    pub venue_id: String,
    pub include_games: bool,
    pub include_trends: bool,
}

/// Request for leaderboard
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LeaderboardRequest {
    pub category: LeaderboardCategory,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub time_period: Option<TimePeriod>,
}

/// Leaderboard categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LeaderboardCategory {
    #[serde(rename = "win_rate")]
    WinRate,
    #[serde(rename = "total_wins")]
    TotalWins,
    #[serde(rename = "skill_rating")]
    SkillRating,
    #[serde(rename = "total_contests")]
    TotalContests,
    #[serde(rename = "longest_streak")]
    LongestStreak,
    #[serde(rename = "best_placement")]
    BestPlacement,
}

impl std::fmt::Display for LeaderboardCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LeaderboardCategory::WinRate => write!(f, "Win Rate"),
            LeaderboardCategory::TotalWins => write!(f, "Total Wins"),
            LeaderboardCategory::SkillRating => write!(f, "Skill Rating"),
            LeaderboardCategory::TotalContests => write!(f, "Total Contests"),
            LeaderboardCategory::LongestStreak => write!(f, "Longest Streak"),
            LeaderboardCategory::BestPlacement => write!(f, "Best Placement"),
        }
    }
}

/// Time periods for analytics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimePeriod {
    #[serde(rename = "all_time")]
    AllTime,
    #[serde(rename = "last_30_days")]
    Last30Days,
    #[serde(rename = "last_7_days")]
    Last7Days,
    #[serde(rename = "last_90_days")]
    Last90Days,
    #[serde(rename = "this_year")]
    ThisYear,
}

/// Leaderboard entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: i32,
    pub player_id: String,
    pub player_handle: String,
    pub player_name: String,
    pub value: f64,
    pub additional_data: Option<serde_json::Value>,
}

/// Leaderboard response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardResponse {
    pub category: LeaderboardCategory,
    pub time_period: TimePeriod,
    pub entries: Vec<LeaderboardEntry>,
    pub total_entries: i32,
    pub last_updated: DateTime<FixedOffset>,
}

// Conversion implementations
impl From<&PlayerStats> for PlayerStatsDto {
    fn from(stats: &PlayerStats) -> Self {
        Self {
            player_id: stats.player_id.clone(),
            player_handle: String::new(), // Will be populated by backend
            player_name: String::new(),   // Will be populated by backend
            total_contests: stats.total_contests,
            total_wins: stats.total_wins,
            total_losses: stats.total_losses,
            win_rate: stats.win_rate,
            average_placement: stats.average_placement,
            best_placement: stats.best_placement,
            skill_rating: stats.skill_rating,
            rating_confidence: stats.rating_confidence,
            total_points: stats.total_points,
            current_streak: stats.current_streak,
            longest_streak: stats.longest_streak,
            last_updated: stats.last_updated,
        }
    }
}

impl From<&ContestStats> for ContestStatsDto {
    fn from(stats: &ContestStats) -> Self {
        Self {
            contest_id: stats.contest_id.clone(),
            contest_name: String::new(), // Will be populated by backend
            participant_count: stats.participant_count,
            completion_count: stats.completion_count,
            completion_rate: stats.completion_rate,
            average_placement: stats.average_placement,
            duration_minutes: stats.duration_minutes,
            most_popular_game: stats.most_popular_game.clone(),
            difficulty_rating: stats.difficulty_rating,
            excitement_rating: stats.excitement_rating,
            last_updated: stats.last_updated,
        }
    }
}

impl From<&GameStats> for GameStatsDto {
    fn from(stats: &GameStats) -> Self {
        Self {
            game_id: stats.game_id.clone(),
            game_name: String::new(), // Will be populated by backend
            total_plays: stats.total_plays,
            unique_players: stats.unique_players,
            average_players: stats.average_players,
            win_rate_distribution: stats
                .win_rate_distribution
                .iter()
                .map(|p| PlayerWinRateDto {
                    player_id: p.player_id.clone(),
                    player_handle: p.player_handle.clone(),
                    wins: p.wins,
                    total_plays: p.total_plays,
                    win_rate: p.win_rate,
                })
                .collect(),
            popularity_trend: stats
                .popularity_trend
                .iter()
                .map(|m| MonthlyPlaysDto {
                    year: m.year,
                    month: m.month,
                    plays: m.plays,
                })
                .collect(),
            average_duration_minutes: stats.average_duration_minutes,
            last_updated: stats.last_updated,
        }
    }
}

impl From<&VenueStats> for VenueStatsDto {
    fn from(stats: &VenueStats) -> Self {
        Self {
            venue_id: stats.venue_id.clone(),
            venue_name: String::new(), // Will be populated by backend
            total_contests: stats.total_contests,
            unique_players: stats.unique_players,
            average_participants: stats.average_participants,
            popular_games: stats
                .popular_games
                .iter()
                .map(|g| GamePopularityDto {
                    game_id: g.game_id.clone(),
                    game_name: g.game_name.clone(),
                    plays: g.plays,
                    popularity_score: g.popularity_score,
                })
                .collect(),
            monthly_contests: stats
                .monthly_contests
                .iter()
                .map(|m| MonthlyContestsDto {
                    year: m.year,
                    month: m.month,
                    contests: m.contests,
                })
                .collect(),
            average_duration_minutes: stats.average_duration_minutes,
            last_updated: stats.last_updated,
        }
    }
}

impl From<&PlatformStats> for PlatformStatsDto {
    fn from(stats: &PlatformStats) -> Self {
        Self {
            total_players: stats.total_players,
            total_contests: stats.total_contests,
            total_games: stats.total_games,
            total_venues: stats.total_venues,
            active_players_30d: stats.active_players_30d,
            active_players_7d: stats.active_players_7d,
            contests_30d: stats.contests_30d,
            average_participants_per_contest: stats.average_participants_per_contest,
            top_games: stats
                .top_games
                .iter()
                .map(|g| GamePopularityDto {
                    game_id: g.game_id.clone(),
                    game_name: g.game_name.clone(),
                    plays: g.plays,
                    popularity_score: g.popularity_score,
                })
                .collect(),
            top_venues: stats
                .top_venues
                .iter()
                .map(|v| VenueActivityDto {
                    venue_id: v.venue_id.clone(),
                    venue_name: v.venue_name.clone(),
                    contests_held: v.contests_held,
                    total_participants: v.total_participants,
                    activity_score: v.activity_score,
                })
                .collect(),
            last_updated: stats.last_updated,
        }
    }
}

impl From<&Achievement> for AchievementDto {
    fn from(achievement: &Achievement) -> Self {
        Self {
            id: achievement.id.clone(),
            name: achievement.name.clone(),
            description: achievement.description.clone(),
            category: match achievement.category {
                AchievementCategory::Wins => AchievementCategoryDto::Wins,
                AchievementCategory::Contests => AchievementCategoryDto::Contests,
                AchievementCategory::Streaks => AchievementCategoryDto::Streaks,
                AchievementCategory::Games => AchievementCategoryDto::Games,
                AchievementCategory::Venues => AchievementCategoryDto::Venues,
                AchievementCategory::Special => AchievementCategoryDto::Special,
            },
            required_value: achievement.required_value,
            current_value: achievement.current_value,
            unlocked: achievement.unlocked,
            unlocked_at: achievement.unlocked_at,
        }
    }
}

impl From<&PlayerAchievements> for PlayerAchievementsDto {
    fn from(achievements: &PlayerAchievements) -> Self {
        Self {
            player_id: achievements.player_id.clone(),
            player_handle: String::new(), // Will be populated by backend
            achievements: achievements.achievements.iter().map(|a| a.into()).collect(),
            total_achievements: achievements.total_achievements,
            unlocked_achievements: achievements.unlocked_achievements,
            completion_percentage: achievements.completion_percentage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use test_log::test;

    #[test]
    fn test_player_stats_dto_creation() {
        let stats = PlayerStats::new("player/123".to_string());
        let dto = PlayerStatsDto::from(&stats);
        assert_eq!(dto.player_id, "player/123");
        assert_eq!(dto.total_contests, 0);
        assert_eq!(dto.win_rate, 0.0);
    }

    #[test]
    fn test_contest_stats_dto_creation() {
        let stats = ContestStats::new("contest/456".to_string());
        let dto = ContestStatsDto::from(&stats);
        assert_eq!(dto.contest_id, "contest/456");
        assert_eq!(dto.participant_count, 0);
        assert_eq!(dto.completion_rate, 0.0);
    }

    #[test]
    fn test_game_stats_dto_creation() {
        let stats = GameStats::new("game/789".to_string());
        let dto = GameStatsDto::from(&stats);
        assert_eq!(dto.game_id, "game/789");
        assert_eq!(dto.total_plays, 0);
        assert_eq!(dto.unique_players, 0);
    }

    #[test]
    fn test_venue_stats_dto_creation() {
        let stats = VenueStats::new("venue/101".to_string());
        let dto = VenueStatsDto::from(&stats);
        assert_eq!(dto.venue_id, "venue/101");
        assert_eq!(dto.total_contests, 0);
        assert_eq!(dto.unique_players, 0);
    }

    #[test]
    fn test_platform_stats_dto_creation() {
        let stats = PlatformStats::new();
        let dto = PlatformStatsDto::from(&stats);
        assert_eq!(dto.total_players, 0);
        assert_eq!(dto.total_contests, 0);
        assert_eq!(dto.total_games, 0);
        assert_eq!(dto.total_venues, 0);
    }

    #[test]
    fn test_head_to_head_record_dto_serde_roundtrip() {
        let record = HeadToHeadRecordDto {
            opponent_id: "player/xyz".to_string(),
            opponent_handle: "curley".to_string(),
            opponent_name: "Curley Jones".to_string(),
            total_contests: 3,
            my_wins: 2,
            opponent_wins: 1,
            my_win_rate: 66.7,
            contest_history: vec![HeadToHeadContestDto {
                contest_id: "contest/1".to_string(),
                contest_name: "The grass shark".to_string(),
                game_name: "Cosmic Encounter".to_string(),
                venue_name: "Blue Room".to_string(),
                my_placement: 1,
                opponent_placement: 2,
                i_won: true,
                contest_date: chrono::Utc::now().fixed_offset(),
                game_id: Some("game/123".to_string()),
                venue_id: Some("venue/456".to_string()),
            }],
        };

        let json = serde_json::to_string(&record).expect("serialize");
        let de: HeadToHeadRecordDto = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(de.opponent_id, record.opponent_id);
        assert_eq!(de.opponent_handle, record.opponent_handle);
        assert_eq!(de.contest_history.len(), 1);
        assert_eq!(de.contest_history[0].venue_name, "Blue Room");
    }
}
