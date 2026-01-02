use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, FixedOffset};


/// Player performance statistics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlayerStats {
    /// Player ID this stats belong to
    pub player_id: String,
    
    /// Total number of contests participated in
    pub total_contests: i32,
    
    /// Total number of wins
    pub total_wins: i32,
    
    /// Total number of losses
    pub total_losses: i32,
    
    /// Win rate as percentage (0.0 - 100.0)
    pub win_rate: f64,
    
    /// Average placement across all contests
    pub average_placement: f64,
    
    /// Best placement achieved
    pub best_placement: i32,
    
    /// Current skill rating (ELO-style)
    pub skill_rating: f64,
    
    /// Rating confidence (higher = more certain)
    pub rating_confidence: f64,
    
    /// Total points earned across all contests
    pub total_points: i32,
    
    /// Current winning streak
    pub current_streak: i32,
    
    /// Longest winning streak
    pub longest_streak: i32,
    
    /// Last updated timestamp
    pub last_updated: DateTime<FixedOffset>,
}

/// Contest analytics and statistics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ContestStats {
    /// Contest ID this stats belong to
    pub contest_id: String,
    
    /// Total number of participants
    pub participant_count: i32,
    
    /// Number of participants who completed the contest
    pub completion_count: i32,
    
    /// Completion rate as percentage
    pub completion_rate: f64,
    
    /// Average placement across all participants
    pub average_placement: f64,
    
    /// Contest duration in minutes
    pub duration_minutes: i32,
    
    /// Most popular game in the contest
    pub most_popular_game: Option<String>,
    
    /// Contest difficulty rating (1-10)
    pub difficulty_rating: f64,
    
    /// Contest excitement rating (based on close finishes)
    pub excitement_rating: f64,
    
    /// Last updated timestamp
    pub last_updated: DateTime<FixedOffset>,
}

/// Game performance analytics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GameStats {
    /// Game ID this stats belong to
    pub game_id: String,
    
    /// Total number of times played
    pub total_plays: i32,
    
    /// Number of unique players who played this game
    pub unique_players: i32,
    
    /// Average number of players per game
    pub average_players: f64,
    
    /// Win rate distribution (who wins most often)
    pub win_rate_distribution: Vec<PlayerWinRate>,
    
    /// Game popularity trend (plays per month)
    pub popularity_trend: Vec<MonthlyPlays>,
    
    /// Average contest duration when this game is played
    pub average_duration_minutes: f64,
    
    /// Last updated timestamp
    pub last_updated: DateTime<FixedOffset>,
}

/// Venue usage analytics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct VenueStats {
    /// Venue ID this stats belong to
    pub venue_id: String,
    
    /// Total number of contests held here
    pub total_contests: i32,
    
    /// Total number of unique players who played here
    pub unique_players: i32,
    
    /// Average participants per contest
    pub average_participants: f64,
    
    /// Most popular games at this venue
    pub popular_games: Vec<GamePopularity>,
    
    /// Monthly contest frequency
    pub monthly_contests: Vec<MonthlyContests>,
    
    /// Average contest duration at this venue
    pub average_duration_minutes: f64,
    
    /// Last updated timestamp
    pub last_updated: DateTime<FixedOffset>,
}

/// Player win rate for a specific game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerWinRate {
    pub player_id: String,
    pub player_handle: String,
    pub wins: i32,
    pub total_plays: i32,
    pub win_rate: f64,
}

/// Monthly plays tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyPlays {
    pub year: i32,
    pub month: u32,
    pub plays: i32,
}

/// Game popularity at a venue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePopularity {
    pub game_id: String,
    pub game_name: String,
    pub plays: i32,
    pub popularity_score: f64,
}

/// Monthly contest frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyContests {
    pub year: i32,
    pub month: u32,
    pub contests: i32,
}

/// Platform-wide analytics
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlatformStats {
    /// Total number of registered players
    pub total_players: i32,
    
    /// Total number of contests held
    pub total_contests: i32,
    
    /// Total number of games in database
    pub total_games: i32,
    
    /// Total number of venues
    pub total_venues: i32,
    
    /// Active players in last 30 days
    pub active_players_30d: i32,
    
    /// Active players in last 7 days
    pub active_players_7d: i32,
    
    /// Contests held in last 30 days
    pub contests_30d: i32,
    
    /// Average participants per contest
    pub average_participants_per_contest: f64,
    
    /// Most popular games overall
    pub top_games: Vec<GamePopularity>,
    
    /// Most active venues
    pub top_venues: Vec<VenueActivity>,
    
    /// Last updated timestamp
    pub last_updated: DateTime<FixedOffset>,
}

/// Venue activity summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueActivity {
    pub venue_id: String,
    pub venue_name: String,
    pub contests_held: i32,
    pub total_participants: i32,
    pub activity_score: f64,
}

/// Achievement definition
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Achievement {
    /// Achievement ID
    pub id: String,
    
    /// Achievement name
    pub name: String,
    
    /// Achievement description
    pub description: String,
    
    /// Achievement category
    pub category: AchievementCategory,
    
    /// Required value to unlock
    pub required_value: i32,
    
    /// Current value for a player
    pub current_value: i32,
    
    /// Whether achievement is unlocked
    pub unlocked: bool,
    
    /// When achievement was unlocked (if applicable)
    pub unlocked_at: Option<DateTime<FixedOffset>>,
}

/// Achievement categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AchievementCategory {
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

/// Player achievement progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerAchievements {
    pub player_id: String,
    pub achievements: Vec<Achievement>,
    pub total_achievements: i32,
    pub unlocked_achievements: i32,
    pub completion_percentage: f64,
}

/// Player ranking in a specific category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRanking {
    pub category: String,
    pub rank: i32,
    pub total_players: i32,
    pub value: f64,
}

/// Player data for achievement calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerData {
    pub player_id: String,
    pub player_handle: String,
    pub total_contests: i32,
    pub total_wins: i32,
    pub unique_games: i32,
    pub unique_venues: i32,
}

impl PlayerStats {
    /// Creates new player stats
    pub fn new(player_id: String) -> Self {
        Self {
            player_id,
            total_contests: 0,
            total_wins: 0,
            total_losses: 0,
            win_rate: 0.0,
            average_placement: 0.0,
            best_placement: 0,
            skill_rating: 1200.0, // Starting ELO rating
            rating_confidence: 0.0,
            total_points: 0,
            current_streak: 0,
            longest_streak: 0,
            last_updated: chrono::Utc::now().into(),
        }
    }
    
    /// Updates win rate based on current wins and total contests
    pub fn update_win_rate(&mut self) {
        if self.total_contests > 0 {
            self.win_rate = (self.total_wins as f64 / self.total_contests as f64) * 100.0;
        }
    }
    
    /// Adds a contest result
    pub fn add_contest_result(&mut self, placement: i32, won: bool) {
        self.total_contests += 1;
        
        if won {
            self.total_wins += 1;
            self.current_streak += 1;
            if self.current_streak > self.longest_streak {
                self.longest_streak = self.current_streak;
            }
        } else {
            self.total_losses += 1;
            self.current_streak = 0;
        }
        
        // Update best placement
        if self.best_placement == 0 || placement < self.best_placement {
            self.best_placement = placement;
        }
        
        // Update average placement
        let total_placements = self.total_contests as f64;
        self.average_placement = ((self.average_placement * (total_placements - 1.0)) + placement as f64) / total_placements;
        
        self.update_win_rate();
        self.last_updated = chrono::Utc::now().into();
    }
}

impl ContestStats {
    /// Creates new contest stats
    pub fn new(contest_id: String) -> Self {
        Self {
            contest_id,
            participant_count: 0,
            completion_count: 0,
            completion_rate: 0.0,
            average_placement: 0.0,
            duration_minutes: 0,
            most_popular_game: None,
            difficulty_rating: 5.0,
            excitement_rating: 5.0,
            last_updated: chrono::Utc::now().into(),
        }
    }
    
    /// Updates completion rate
    pub fn update_completion_rate(&mut self) {
        if self.participant_count > 0 {
            self.completion_rate = (self.completion_count as f64 / self.participant_count as f64) * 100.0;
        }
    }
}

impl GameStats {
    /// Creates new game stats
    pub fn new(game_id: String) -> Self {
        Self {
            game_id,
            total_plays: 0,
            unique_players: 0,
            average_players: 0.0,
            win_rate_distribution: Vec::new(),
            popularity_trend: Vec::new(),
            average_duration_minutes: 0.0,
            last_updated: chrono::Utc::now().into(),
        }
    }
}

impl VenueStats {
    /// Creates new venue stats
    pub fn new(venue_id: String) -> Self {
        Self {
            venue_id,
            total_contests: 0,
            unique_players: 0,
            average_participants: 0.0,
            popular_games: Vec::new(),
            monthly_contests: Vec::new(),
            average_duration_minutes: 0.0,
            last_updated: chrono::Utc::now().into(),
        }
    }
}

impl PlatformStats {
    /// Creates new platform stats
    pub fn new() -> Self {
        Self {
            total_players: 0,
            total_contests: 0,
            total_games: 0,
            total_venues: 0,
            active_players_30d: 0,
            active_players_7d: 0,
            contests_30d: 0,
            average_participants_per_contest: 0.0,
            top_games: Vec::new(),
            top_venues: Vec::new(),
            last_updated: chrono::Utc::now().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use test_log::test;

    #[test]
    fn test_player_stats_creation() {
        let stats = PlayerStats::new("player/123".to_string());
        assert_eq!(stats.player_id, "player/123");
        assert_eq!(stats.total_contests, 0);
        assert_eq!(stats.win_rate, 0.0);
        assert_eq!(stats.skill_rating, 1200.0);
    }

    #[test]
    fn test_player_stats_add_contest_result() {
        let mut stats = PlayerStats::new("player/123".to_string());
        
        // Add a win
        stats.add_contest_result(1, true);
        assert_eq!(stats.total_contests, 1);
        assert_eq!(stats.total_wins, 1);
        assert_eq!(stats.win_rate, 100.0);
        assert_eq!(stats.best_placement, 1);
        assert_eq!(stats.current_streak, 1);
        
        // Add a loss
        stats.add_contest_result(3, false);
        assert_eq!(stats.total_contests, 2);
        assert_eq!(stats.total_wins, 1);
        assert_eq!(stats.total_losses, 1);
        assert_eq!(stats.win_rate, 50.0);
        assert_eq!(stats.current_streak, 0);
        assert_eq!(stats.average_placement, 2.0);
    }

    #[test]
    fn test_contest_stats_creation() {
        let stats = ContestStats::new("contest/456".to_string());
        assert_eq!(stats.contest_id, "contest/456");
        assert_eq!(stats.participant_count, 0);
        assert_eq!(stats.completion_rate, 0.0);
    }

    #[test]
    fn test_contest_stats_completion_rate() {
        let mut stats = ContestStats::new("contest/456".to_string());
        stats.participant_count = 10;
        stats.completion_count = 8;
        stats.update_completion_rate();
        assert_eq!(stats.completion_rate, 80.0);
    }

    #[test]
    fn test_game_stats_creation() {
        let stats = GameStats::new("game/789".to_string());
        assert_eq!(stats.game_id, "game/789");
        assert_eq!(stats.total_plays, 0);
        assert_eq!(stats.unique_players, 0);
    }

    #[test]
    fn test_venue_stats_creation() {
        let stats = VenueStats::new("venue/101".to_string());
        assert_eq!(stats.venue_id, "venue/101");
        assert_eq!(stats.total_contests, 0);
        assert_eq!(stats.unique_players, 0);
    }

    #[test]
    fn test_platform_stats_creation() {
        let stats = PlatformStats::new();
        assert_eq!(stats.total_players, 0);
        assert_eq!(stats.total_contests, 0);
        assert_eq!(stats.total_games, 0);
        assert_eq!(stats.total_venues, 0);
    }
} 