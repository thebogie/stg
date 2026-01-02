use std::collections::HashMap;
use chrono::{DateTime, FixedOffset, Datelike};
use shared::{
    models::analytics::*,
    dto::analytics::*,
};

/// Core analytics calculation engine
#[derive(Clone)]
pub struct AnalyticsEngine {
    /// ELO rating system configuration
    elo_config: EloConfig,
}

/// ELO rating system configuration
#[derive(Debug, Clone)]
pub struct EloConfig {
    /// Starting ELO rating for new players
    pub starting_rating: f64,
    /// K-factor for rating adjustments (higher = more volatile)
    pub k_factor: f64,
    /// Rating deviation factor for uncertainty
    pub rating_deviation: f64,
    /// Minimum rating change
    pub min_rating_change: f64,
    /// Maximum rating change
    pub max_rating_change: f64,
}

impl Default for EloConfig {
    fn default() -> Self {
        Self {
            starting_rating: 1200.0,
            k_factor: 32.0,
            rating_deviation: 200.0,
            min_rating_change: -50.0,
            max_rating_change: 50.0,
        }
    }
}

impl AnalyticsEngine {
    /// Creates a new analytics engine with default configuration
    pub fn new() -> Self {
        Self {
            elo_config: EloConfig::default(),
        }
    }

    /// Creates a new analytics engine with custom configuration
    pub fn with_config(config: EloConfig) -> Self {
        Self { elo_config: config }
    }

    /// Calculates player statistics from contest results
    pub fn calculate_player_stats(&self, player_id: &str, contest_results: Vec<ContestResult>) -> PlayerStats {
        let mut stats = PlayerStats::new(player_id.to_string());
        
        for result in contest_results {
            let won = result.placement == 1;
            stats.add_contest_result(result.placement, won);
            
            // Update skill rating using ELO
            if let Some(opponent_rating) = result.average_opponent_rating {
                let rating_change = self.calculate_elo_change(
                    stats.skill_rating,
                    opponent_rating,
                    won,
                    result.contest_difficulty.unwrap_or(1.0)
                );
                stats.skill_rating += rating_change;
                stats.skill_rating = stats.skill_rating.max(100.0); // Minimum rating
            }
        }
        
        // Calculate rating confidence based on number of games
        stats.rating_confidence = self.calculate_rating_confidence(stats.total_contests);
        
        stats
    }

    /// Calculates contest statistics
    pub fn calculate_contest_stats(&self, contest_id: &str, participants: Vec<ContestParticipant>) -> ContestStats {
        let mut stats = ContestStats::new(contest_id.to_string());
        
        stats.participant_count = participants.len() as i32;
        stats.completion_count = participants.iter().filter(|p| p.completed).count() as i32;
        stats.update_completion_rate();
        
        if !participants.is_empty() {
            let total_placement: f64 = participants.iter().map(|p| p.placement as f64).sum();
            stats.average_placement = total_placement / participants.len() as f64;
        }
        
        // Calculate difficulty rating based on participant skill levels
        if !participants.is_empty() {
            let avg_skill: f64 = participants.iter().map(|p| p.skill_rating).sum::<f64>() / participants.len() as f64;
            stats.difficulty_rating = self.calculate_difficulty_rating(avg_skill, participants.len());
        }
        
        // Calculate excitement rating based on close finishes
        stats.excitement_rating = self.calculate_excitement_rating(&participants);
        
        stats
    }

    /// Calculates game statistics
    pub fn calculate_game_stats(&self, game_id: &str, game_plays: Vec<GamePlay>) -> GameStats {
        let mut stats = GameStats::new(game_id.to_string());
        
        stats.total_plays = game_plays.len() as i32;
        
        // Count unique players
        let unique_players: std::collections::HashSet<String> = game_plays.iter()
            .map(|play| play.player_id.clone())
            .collect();
        stats.unique_players = unique_players.len() as i32;
        
        // Calculate average players per game
        if !game_plays.is_empty() {
            let total_players: f64 = game_plays.iter().map(|play| play.player_count as f64).sum();
            stats.average_players = total_players / game_plays.len() as f64;
        }
        
        // Calculate win rate distribution
        stats.win_rate_distribution = self.calculate_win_rate_distribution(&game_plays);
        
        // Calculate popularity trend
        stats.popularity_trend = self.calculate_popularity_trend(&game_plays);
        
        // Calculate average duration
        if !game_plays.is_empty() {
            let total_duration: f64 = game_plays.iter().map(|play| play.duration_minutes as f64).sum();
            stats.average_duration_minutes = total_duration / game_plays.len() as f64;
        }
        
        stats
    }

    /// Calculates venue statistics
    pub fn calculate_venue_stats(&self, venue_id: &str, venue_contests: Vec<VenueContest>) -> VenueStats {
        let mut stats = VenueStats::new(venue_id.to_string());
        
        stats.total_contests = venue_contests.len() as i32;
        
        // Count unique players
        let unique_players: std::collections::HashSet<String> = venue_contests.iter()
            .flat_map(|contest| contest.participant_ids.clone())
            .collect();
        stats.unique_players = unique_players.len() as i32;
        
        // Calculate average participants per contest
        if !venue_contests.is_empty() {
            let total_participants: f64 = venue_contests.iter().map(|contest| contest.participant_count as f64).sum();
            stats.average_participants = total_participants / venue_contests.len() as f64;
        }
        
        // Calculate popular games
        stats.popular_games = self.calculate_popular_games(&venue_contests);
        
        // Calculate monthly contest frequency
        stats.monthly_contests = self.calculate_monthly_contests(&venue_contests);
        
        // Calculate average duration
        if !venue_contests.is_empty() {
            let total_duration: f64 = venue_contests.iter().map(|contest| contest.duration_minutes as f64).sum();
            stats.average_duration_minutes = total_duration / venue_contests.len() as f64;
        }
        
        stats
    }

    /// Calculates ELO rating change
    pub fn calculate_elo_change(&self, player_rating: f64, opponent_rating: f64, won: bool, difficulty: f64) -> f64 {
        let expected_score = 1.0 / (1.0 + 10.0_f64.powf((opponent_rating - player_rating) / 400.0));
        let actual_score = if won { 1.0 } else { 0.0 };
        
        let rating_change = self.elo_config.k_factor * (actual_score - expected_score) * difficulty;
        
        // Clamp rating change
        rating_change.max(self.elo_config.min_rating_change)
            .min(self.elo_config.max_rating_change)
    }

    /// Calculates rating confidence based on number of games
    pub fn calculate_rating_confidence(&self, total_contests: i32) -> f64 {
        match total_contests {
            0 => 0.0,
            1..=5 => 0.2,
            6..=10 => 0.4,
            11..=20 => 0.6,
            21..=50 => 0.8,
            _ => 1.0,
        }
    }

    /// Calculates difficulty rating
    pub fn calculate_difficulty_rating(&self, average_skill: f64, participant_count: usize) -> f64 {
        let skill_factor = (average_skill - self.elo_config.starting_rating) / 400.0;
        let participant_factor = (participant_count as f64 - 2.0) / 8.0; // Normalize to 0-1
        
        let difficulty = 5.0 + skill_factor * 3.0 + participant_factor * 2.0;
        difficulty.max(1.0).min(10.0)
    }

    /// Calculates excitement rating based on close finishes
    pub fn calculate_excitement_rating(&self, participants: &[ContestParticipant]) -> f64 {
        if participants.len() < 2 {
            return 5.0;
        }
        
        let mut sorted = participants.to_vec();
        sorted.sort_by_key(|p| p.placement);
        
        // Check for close finishes (1st and 2nd place close)
        let first_place = sorted.first().unwrap();
        let second_place = sorted.get(1).unwrap();
        
        let score_difference = (first_place.score - second_place.score).abs();
        let max_score = first_place.score.max(second_place.score);
        
        let closeness_factor = if max_score > 0.0 {
            1.0 - (score_difference / max_score)
        } else {
            1.0
        };
        
        let excitement = 5.0 + closeness_factor * 5.0;
        excitement.max(1.0).min(10.0)
    }

    /// Calculates win rate distribution for a game
    pub fn calculate_win_rate_distribution(&self, game_plays: &[GamePlay]) -> Vec<PlayerWinRate> {
        let mut player_stats: HashMap<String, (i32, i32)> = HashMap::new();
        
        for play in game_plays {
            let entry = player_stats.entry(play.player_id.clone()).or_insert((0, 0));
            entry.1 += 1; // total plays
            if play.won {
                entry.0 += 1; // wins
            }
        }
        
        player_stats.into_iter()
            .map(|(player_id, (wins, total_plays))| {
                let win_rate = if total_plays > 0 {
                    (wins as f64 / total_plays as f64) * 100.0
                } else {
                    0.0
                };
                
                PlayerWinRate {
                    player_id,
                    player_handle: String::new(), // Will be populated by repository
                    wins,
                    total_plays,
                    win_rate,
                }
            })
            .collect()
    }

    /// Calculates popularity trend for a game
    pub fn calculate_popularity_trend(&self, game_plays: &[GamePlay]) -> Vec<MonthlyPlays> {
        let mut monthly_plays: HashMap<(i32, u32), i32> = HashMap::new();
        
        for play in game_plays {
            let year = play.played_at.year();
            let month = play.played_at.month();
            *monthly_plays.entry((year, month)).or_insert(0) += 1;
        }
        
        let mut trend: Vec<MonthlyPlays> = monthly_plays.into_iter()
            .map(|((year, month), plays)| MonthlyPlays { year, month, plays })
            .collect();
        
        trend.sort_by_key(|m| (m.year, m.month));
        trend
    }

    /// Calculates popular games at a venue
    pub fn calculate_popular_games(&self, venue_contests: &[VenueContest]) -> Vec<GamePopularity> {
        let mut game_stats: HashMap<String, (i32, String)> = HashMap::new();
        
        for contest in venue_contests {
            for game_id in &contest.game_ids {
                let entry = game_stats.entry(game_id.clone()).or_insert((0, String::new()));
                entry.0 += 1; // plays
            }
        }
        
        let mut popular_games: Vec<GamePopularity> = game_stats.into_iter()
            .map(|(game_id, (plays, game_name))| {
                let popularity_score = plays as f64 / venue_contests.len() as f64;
                
                GamePopularity {
                    game_id,
                    game_name,
                    plays,
                    popularity_score,
                }
            })
            .collect();
        
        popular_games.sort_by(|a, b| b.plays.cmp(&a.plays));
        popular_games
    }

    /// Calculates monthly contest frequency for a venue
    pub fn calculate_monthly_contests(&self, venue_contests: &[VenueContest]) -> Vec<MonthlyContests> {
        let mut monthly_contests: HashMap<(i32, u32), i32> = HashMap::new();
        
        for contest in venue_contests {
            let year = contest.contest_date.year();
            let month = contest.contest_date.month();
            *monthly_contests.entry((year, month)).or_insert(0) += 1;
        }
        
        let mut trend: Vec<MonthlyContests> = monthly_contests.into_iter()
            .map(|((year, month), contests)| MonthlyContests { year, month, contests })
            .collect();
        
        trend.sort_by_key(|m| (m.year, m.month));
        trend
    }

    /// Generates leaderboard entries
    pub fn generate_leaderboard(&self, players: Vec<PlayerStats>, category: LeaderboardCategory, limit: usize) -> Vec<LeaderboardEntry> {
        let mut entries: Vec<LeaderboardEntry> = players.into_iter()
            .enumerate()
            .map(|(index, stats)| {
                let value = match category {
                    LeaderboardCategory::WinRate => stats.win_rate,
                    LeaderboardCategory::TotalWins => stats.total_wins as f64,
                    LeaderboardCategory::SkillRating => stats.skill_rating,
                    LeaderboardCategory::TotalContests => stats.total_contests as f64,
                    LeaderboardCategory::LongestStreak => stats.longest_streak as f64,
                    LeaderboardCategory::BestPlacement => stats.best_placement as f64,
                };
                
                LeaderboardEntry {
                    rank: (index + 1) as i32,
                    player_id: stats.player_id,
                    player_handle: String::new(), // Will be populated by repository
                    player_name: String::new(),   // Will be populated by repository
                    value,
                    additional_data: None,
                }
            })
            .collect();
        
        // Sort by value (descending for most categories, ascending for best placement)
        match category {
            LeaderboardCategory::BestPlacement => {
                entries.sort_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal));
            }
            _ => {
                entries.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
            }
        }
        
        entries.into_iter().take(limit).collect()
    }
}

// Data structures for calculations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContestResult {
    pub contest_id: String,
    pub placement: i32,
    pub score: f64,
    pub average_opponent_rating: Option<f64>,
    pub contest_difficulty: Option<f64>,
    pub contest_date: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContestParticipant {
    pub player_id: String,
    pub placement: i32,
    pub score: f64,
    pub skill_rating: f64,
    pub completed: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GamePlay {
    pub player_id: String,
    pub player_count: i32,
    pub won: bool,
    pub duration_minutes: i32,
    pub played_at: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VenueContest {
    pub contest_id: String,
    pub participant_ids: Vec<String>,
    pub participant_count: i32,
    pub game_ids: Vec<String>,
    pub duration_minutes: i32,
    pub contest_date: DateTime<FixedOffset>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use test_log::test;

    #[test]
    fn test_analytics_engine_creation() {
        let engine = AnalyticsEngine::new();
        assert_eq!(engine.elo_config.starting_rating, 1200.0);
        assert_eq!(engine.elo_config.k_factor, 32.0);
    }

    #[test]
    fn test_elo_calculation() {
        let engine = AnalyticsEngine::new();
        
        // Test win against equal opponent
        let change = engine.calculate_elo_change(1200.0, 1200.0, true, 1.0);
        assert!(change > 0.0);
        
        // Test loss against equal opponent
        let change = engine.calculate_elo_change(1200.0, 1200.0, false, 1.0);
        assert!(change < 0.0);
    }

    #[test]
    fn test_rating_confidence_calculation() {
        let engine = AnalyticsEngine::new();
        
        assert_eq!(engine.calculate_rating_confidence(0), 0.0);
        assert_eq!(engine.calculate_rating_confidence(1), 0.2);
        assert_eq!(engine.calculate_rating_confidence(10), 0.4);
        assert_eq!(engine.calculate_rating_confidence(100), 1.0);
    }

    #[test]
    fn test_difficulty_rating_calculation() {
        let engine = AnalyticsEngine::new();
        
        let difficulty = engine.calculate_difficulty_rating(1200.0, 4);
        assert!(difficulty >= 1.0 && difficulty <= 10.0);
    }

    #[test]
    fn test_excitement_rating_calculation() {
        let engine = AnalyticsEngine::new();
        
        let participants = vec![
            ContestParticipant {
                player_id: "player1".to_string(),
                placement: 1,
                score: 100.0,
                skill_rating: 1200.0,
                completed: true,
            },
            ContestParticipant {
                player_id: "player2".to_string(),
                placement: 2,
                score: 99.0,
                skill_rating: 1200.0,
                completed: true,
            },
        ];
        
        let excitement = engine.calculate_excitement_rating(&participants);
        assert!(excitement >= 1.0 && excitement <= 10.0);
    }
} 