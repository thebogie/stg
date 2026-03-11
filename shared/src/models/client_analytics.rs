use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Optimized contest data for client-side analytics
/// Strips unnecessary fields and flattens relationships for fast access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientContest {
    pub id: String,
    pub name: String,
    pub start: DateTime<FixedOffset>,
    pub end: DateTime<FixedOffset>,
    pub game: ClientGame,
    pub venue: ClientVenue,
    pub participants: Vec<ClientParticipant>,
    pub my_result: ClientResult,
}

/// Simplified game data for client storage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientGame {
    pub id: String,
    pub name: String,
    pub year_published: Option<i32>,
}

/// Simplified venue data for client storage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientVenue {
    pub id: String,
    pub name: String,
    pub display_name: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
}

/// Participant result data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientParticipant {
    pub player_id: String,
    pub handle: String,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub place: i32,
    pub result: String, // "won", "lost", "tied"
}

/// Current user's result in this contest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientResult {
    pub place: i32,
    pub result: String,
    pub points: Option<i32>,
}

/// Client-side analytics cache with pre-computed values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientAnalyticsCache {
    pub last_updated: DateTime<FixedOffset>,
    pub player_id: String,

    // Core statistics (computed once, accessed frequently)
    pub core_stats: CoreStats,

    // Raw data for real-time calculations
    pub contests: Vec<ClientContest>,

    // Pre-computed lookups for fast access
    pub game_lookup: HashMap<String, ClientGame>,
    pub venue_lookup: HashMap<String, ClientVenue>,
    pub opponent_lookup: HashMap<String, ClientOpponent>,

    // Cache metadata
    pub cache_version: String,
    pub data_size_bytes: usize,
}

/// Core statistics computed once and cached
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoreStats {
    pub total_contests: i32,
    pub total_wins: i32,
    pub total_losses: i32,
    pub win_rate: f64,
    pub average_placement: f64,
    pub best_placement: i32,
    pub worst_placement: i32,
    pub current_streak: i32,
    pub longest_streak: i32,
    pub skill_rating: f64,
    pub total_points: i32,
}

/// Opponent data for fast lookups
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientOpponent {
    pub player_id: String,
    pub handle: String,
    pub name: String,
    pub contests_against: i32,
    pub wins_against: i32,
    pub losses_against: i32,
    pub win_rate_against: f64,
    pub last_played: DateTime<FixedOffset>,
}

/// Analytics query parameters for client-side filtering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalyticsQuery {
    pub date_range: Option<DateRange>,
    pub games: Option<Vec<String>>,
    pub venues: Option<Vec<String>>,
    pub opponents: Option<Vec<String>>,
    pub min_players: Option<i32>,
    pub max_players: Option<i32>,
}

/// Date range for filtering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DateRange {
    pub start: DateTime<FixedOffset>,
    pub end: DateTime<FixedOffset>,
}

/// Real-time computed analytics result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputedAnalytics {
    pub query: AnalyticsQuery,
    pub contests: Vec<ClientContest>,
    pub stats: CoreStats,
    pub game_performance: Vec<GamePerformance>,
    pub opponent_performance: Vec<OpponentPerformance>,
    pub trends: Vec<PerformanceTrend>,
    pub computed_at: DateTime<FixedOffset>,
}

/// Game performance computed in real-time
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GamePerformance {
    pub game: ClientGame,
    pub total_plays: i32,
    pub wins: i32,
    pub losses: i32,
    pub win_rate: f64,
    pub average_placement: f64,
    pub best_placement: i32,
    pub worst_placement: i32,
    pub last_played: DateTime<FixedOffset>,
    pub days_since_last_play: i64,
    pub favorite_venue: Option<ClientVenue>,
}

/// Opponent performance computed in real-time
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpponentPerformance {
    pub opponent: ClientOpponent,
    pub head_to_head: HeadToHeadStats,
}

/// Head-to-head statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeadToHeadStats {
    pub total_contests: i32,
    pub my_wins: i32,
    pub opponent_wins: i32,
    pub my_win_rate: f64,
    pub contest_history: Vec<ClientContest>,
}

/// Performance trend over time
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerformanceTrend {
    pub period: String, // "2024-01", "2024-Q1", "2024"
    pub contests_played: i32,
    pub wins: i32,
    pub win_rate: f64,
    pub average_placement: f64,
    pub skill_rating: f64,
}

impl ClientAnalyticsCache {
    /// Creates a new cache instance
    pub fn new(player_id: String) -> Self {
        Self {
            last_updated: chrono::Utc::now().fixed_offset(),
            player_id,
            core_stats: CoreStats::default(),
            contests: Vec::new(),
            game_lookup: HashMap::new(),
            venue_lookup: HashMap::new(),
            opponent_lookup: HashMap::new(),
            cache_version: "1.0.0".to_string(),
            data_size_bytes: 0,
        }
    }

    /// Computes core statistics from raw contest data
    pub fn compute_core_stats(&mut self) {
        if self.contests.is_empty() {
            return;
        }

        let mut total_contests = 0;
        let mut total_wins = 0;
        let mut total_losses = 0;
        let mut total_placement = 0;
        let mut best_placement = i32::MAX;
        let mut worst_placement = 0;
        let current_streak;
        let mut longest_streak = 0;
        let mut temp_streak: i32 = 0;

        // Sort contests by date (newest first) for streak calculation
        let mut sorted_contests = self.contests.clone();
        sorted_contests.sort_by(|a, b| b.start.cmp(&a.start));

        for contest in &sorted_contests {
            total_contests += 1;

            match contest.my_result.result.as_str() {
                "won" => {
                    total_wins += 1;
                    temp_streak = temp_streak.max(0) + 1;
                }
                "lost" => {
                    total_losses += 1;
                    temp_streak = temp_streak.min(0) - 1;
                }
                _ => temp_streak = 0,
            }

            let placement = contest.my_result.place;
            total_placement += placement;
            best_placement = best_placement.min(placement);
            worst_placement = worst_placement.max(placement);

            longest_streak = longest_streak.max(temp_streak.abs());
        }

        // Set current streak (most recent)
        current_streak = temp_streak;

        self.core_stats = CoreStats {
            total_contests,
            total_wins,
            total_losses,
            win_rate: if total_contests > 0 {
                (total_wins as f64 / total_contests as f64) * 100.0
            } else {
                0.0
            },
            average_placement: if total_contests > 0 {
                total_placement as f64 / total_contests as f64
            } else {
                0.0
            },
            best_placement: if best_placement == i32::MAX {
                0
            } else {
                best_placement
            },
            worst_placement,
            current_streak,
            longest_streak,
            skill_rating: 1000.0, // Placeholder - implement ELO calculation
            total_points: 0,      // Placeholder - implement points system
        };
    }

    /// Builds lookup tables for fast access
    pub fn build_lookups(&mut self) {
        self.game_lookup.clear();
        self.venue_lookup.clear();
        self.opponent_lookup.clear();

        for contest in &self.contests {
            // Game lookup
            if !self.game_lookup.contains_key(&contest.game.id) {
                self.game_lookup
                    .insert(contest.game.id.clone(), contest.game.clone());
            }

            // Venue lookup
            if !self.venue_lookup.contains_key(&contest.venue.id) {
                self.venue_lookup
                    .insert(contest.venue.id.clone(), contest.venue.clone());
            }

            // Opponent lookup
            for participant in &contest.participants {
                if participant.player_id != self.player_id
                    && !self.opponent_lookup.contains_key(&participant.player_id)
                {
                    let opponent = ClientOpponent {
                        player_id: participant.player_id.clone(),
                        handle: participant.handle.clone(),
                        name: format!(
                            "{} {}",
                            participant.firstname.as_deref().unwrap_or(""),
                            participant.lastname.as_deref().unwrap_or("")
                        )
                        .trim()
                        .to_string(),
                        contests_against: 0, // Will be computed separately
                        wins_against: 0,
                        losses_against: 0,
                        win_rate_against: 0.0,
                        last_played: contest.start,
                    };
                    self.opponent_lookup
                        .insert(participant.player_id.clone(), opponent);
                }
            }
        }

        // Compute opponent statistics
        self.compute_opponent_stats();
    }

    /// Computes opponent statistics
    fn compute_opponent_stats(&mut self) {
        for opponent in self.opponent_lookup.values_mut() {
            let mut contests_against = 0;
            let mut wins_against = 0;
            let mut losses_against = 0;
            let mut last_played = chrono::Utc::now().fixed_offset();

            for contest in &self.contests {
                if contest
                    .participants
                    .iter()
                    .any(|p| p.player_id == opponent.player_id)
                {
                    contests_against += 1;
                    last_played = last_played.max(contest.start);

                    if contest.my_result.result == "won" {
                        wins_against += 1;
                    } else if contest.my_result.result == "lost" {
                        losses_against += 1;
                    }
                }
            }

            opponent.contests_against = contests_against;
            opponent.wins_against = wins_against;
            opponent.losses_against = losses_against;
            opponent.win_rate_against = if contests_against > 0 {
                (wins_against as f64 / contests_against as f64) * 100.0
            } else {
                0.0
            };
            opponent.last_played = last_played;
        }
    }

    /// Executes a real-time analytics query
    pub fn query_analytics(&self, query: AnalyticsQuery) -> ComputedAnalytics {
        let mut filtered_contests = self.contests.clone();

        // Apply date range filter
        if let Some(date_range) = &query.date_range {
            filtered_contests.retain(|c| c.start >= date_range.start && c.start <= date_range.end);
        }

        // Apply game filter
        if let Some(games) = &query.games {
            filtered_contests.retain(|c| games.contains(&c.game.id));
        }

        // Apply venue filter
        if let Some(venues) = &query.venues {
            filtered_contests.retain(|c| venues.contains(&c.venue.id));
        }

        // Apply opponent filter
        if let Some(opponents) = &query.opponents {
            filtered_contests.retain(|c| {
                c.participants
                    .iter()
                    .any(|p| opponents.contains(&p.player_id))
            });
        }

        // Apply player count filter
        if let Some(min_players) = query.min_players {
            filtered_contests.retain(|c| c.participants.len() >= min_players as usize);
        }
        if let Some(max_players) = query.max_players {
            filtered_contests.retain(|c| c.participants.len() <= max_players as usize);
        }

        // Compute statistics from filtered data
        let stats = self.compute_stats_from_contests(&filtered_contests);
        let game_performance = self.compute_game_performance(&filtered_contests);
        let opponent_performance = self.compute_opponent_performance(&filtered_contests);
        let trends = self.compute_trends(&filtered_contests);

        ComputedAnalytics {
            query,
            contests: filtered_contests,
            stats,
            game_performance,
            opponent_performance,
            trends,
            computed_at: chrono::Utc::now().fixed_offset(),
        }
    }

    /// Computes statistics from a subset of contests
    fn compute_stats_from_contests(&self, contests: &[ClientContest]) -> CoreStats {
        if contests.is_empty() {
            return CoreStats::default();
        }

        let mut total_contests = 0;
        let mut total_wins = 0;
        let mut total_losses = 0;
        let mut total_placement = 0;
        let mut best_placement = i32::MAX;
        let mut worst_placement = 0;

        for contest in contests {
            total_contests += 1;

            match contest.my_result.result.as_str() {
                "won" => total_wins += 1,
                "lost" => total_losses += 1,
                _ => {}
            }

            let placement = contest.my_result.place;
            total_placement += placement;
            best_placement = best_placement.min(placement);
            worst_placement = worst_placement.max(placement);
        }

        CoreStats {
            total_contests,
            total_wins,
            total_losses,
            win_rate: if total_contests > 0 {
                (total_wins as f64 / total_contests as f64) * 100.0
            } else {
                0.0
            },
            average_placement: if total_contests > 0 {
                total_placement as f64 / total_contests as f64
            } else {
                0.0
            },
            best_placement: if best_placement == i32::MAX {
                0
            } else {
                best_placement
            },
            worst_placement,
            current_streak: 0, // Would need to compute from full dataset
            longest_streak: 0, // Would need to compute from full dataset
            skill_rating: 1000.0,
            total_points: 0,
        }
    }

    /// Computes game performance from contests
    fn compute_game_performance(&self, contests: &[ClientContest]) -> Vec<GamePerformance> {
        let mut game_stats: HashMap<String, GamePerformance> = HashMap::new();

        for contest in contests {
            let game_id = contest.game.id.clone();
            let entry = game_stats
                .entry(game_id)
                .or_insert_with(|| GamePerformance {
                    game: contest.game.clone(),
                    total_plays: 0,
                    wins: 0,
                    losses: 0,
                    win_rate: 0.0,
                    average_placement: 0.0,
                    best_placement: i32::MAX,
                    worst_placement: 0,
                    last_played: contest.start,
                    days_since_last_play: 0,
                    favorite_venue: None,
                });

            entry.total_plays += 1;
            entry.last_played = entry.last_played.max(contest.start);

            match contest.my_result.result.as_str() {
                "won" => entry.wins += 1,
                "lost" => entry.losses += 1,
                _ => {}
            }

            let placement = contest.my_result.place;
            entry.best_placement = entry.best_placement.min(placement);
            entry.worst_placement = entry.worst_placement.max(placement);
        }

        // Calculate derived values
        for performance in game_stats.values_mut() {
            performance.win_rate = if performance.total_plays > 0 {
                (performance.wins as f64 / performance.total_plays as f64) * 100.0
            } else {
                0.0
            };

            performance.days_since_last_play = chrono::Utc::now()
                .fixed_offset()
                .signed_duration_since(performance.last_played)
                .num_days();

            // Compute average placement across contests for this game
            if performance.total_plays > 0 {
                let mut placement_sum: i64 = 0;
                let mut count: i64 = 0;
                for contest in contests {
                    if contest.game.id == performance.game.id {
                        placement_sum += contest.my_result.place as i64;
                        count += 1;
                    }
                }
                if count > 0 {
                    performance.average_placement = placement_sum as f64 / count as f64;
                }
            }

            // Find favorite venue (most played at)
            let mut venue_counts: HashMap<String, usize> = HashMap::new();
            for contest in contests {
                if contest.game.id == performance.game.id {
                    *venue_counts.entry(contest.venue.id.clone()).or_default() += 1;
                }
            }

            if let Some((favorite_venue_id, _)) =
                venue_counts.iter().max_by_key(|(_, &count)| count)
            {
                if let Some(venue) = self.venue_lookup.get(favorite_venue_id) {
                    performance.favorite_venue = Some(venue.clone());
                }
            }
        }

        let mut result: Vec<_> = game_stats.into_values().collect();
        result.sort_by(|a, b| {
            b.total_plays
                .cmp(&a.total_plays)
                .then(b.win_rate.partial_cmp(&a.win_rate).unwrap())
        });
        result
    }

    /// Computes opponent performance from contests
    fn compute_opponent_performance(&self, contests: &[ClientContest]) -> Vec<OpponentPerformance> {
        let mut opponent_stats: HashMap<String, HeadToHeadStats> = HashMap::new();

        for contest in contests {
            for participant in &contest.participants {
                if participant.player_id != self.player_id {
                    let entry = opponent_stats
                        .entry(participant.player_id.clone())
                        .or_insert_with(|| HeadToHeadStats {
                            total_contests: 0,
                            my_wins: 0,
                            opponent_wins: 0,
                            my_win_rate: 0.0,
                            contest_history: Vec::new(),
                        });

                    entry.total_contests += 1;
                    entry.contest_history.push(contest.clone());

                    match contest.my_result.result.as_str() {
                        "won" => entry.my_wins += 1,
                        "lost" => entry.opponent_wins += 1,
                        _ => {}
                    }
                }
            }
        }

        // Calculate win rates and sort by most played
        for stats in opponent_stats.values_mut() {
            stats.my_win_rate = if stats.total_contests > 0 {
                (stats.my_wins as f64 / stats.total_contests as f64) * 100.0
            } else {
                0.0
            };
        }

        let mut result: Vec<_> = opponent_stats
            .into_iter()
            .filter_map(|(opponent_id, stats)| {
                self.opponent_lookup
                    .get(&opponent_id)
                    .map(|opponent| OpponentPerformance {
                        opponent: opponent.clone(),
                        head_to_head: stats,
                    })
            })
            .collect();

        result.sort_by(|a, b| {
            b.head_to_head
                .total_contests
                .cmp(&a.head_to_head.total_contests)
        });
        result
    }

    /// Computes performance trends over time
    fn compute_trends(&self, contests: &[ClientContest]) -> Vec<PerformanceTrend> {
        let mut monthly_stats: HashMap<String, (i32, i32, f64)> = HashMap::new();

        for contest in contests {
            let month_key = contest.start.format("%Y-%m").to_string();
            let entry = monthly_stats.entry(month_key).or_insert((0, 0, 0.0));

            entry.0 += 1; // contests_played
            if contest.my_result.result == "won" {
                entry.1 += 1; // wins
            }
            entry.2 += contest.my_result.place as f64; // total placement
        }

        let mut trends: Vec<_> = monthly_stats
            .into_iter()
            .map(|(period, (contests_played, wins, total_placement))| {
                let win_rate = if contests_played > 0 {
                    (wins as f64 / contests_played as f64) * 100.0
                } else {
                    0.0
                };
                let average_placement = if contests_played > 0 {
                    total_placement / contests_played as f64
                } else {
                    0.0
                };

                PerformanceTrend {
                    period,
                    contests_played,
                    wins,
                    win_rate,
                    average_placement,
                    skill_rating: 1000.0, // Placeholder
                }
            })
            .collect();

        trends.sort_by(|a, b| b.period.cmp(&a.period));
        trends
    }

    /// Estimates the size of cached data
    pub fn estimate_size(&self) -> usize {
        let contests_size = self.contests.len() * std::mem::size_of::<ClientContest>();
        let lookups_size = self.game_lookup.len() * std::mem::size_of::<ClientGame>()
            + self.venue_lookup.len() * std::mem::size_of::<ClientVenue>()
            + self.opponent_lookup.len() * std::mem::size_of::<ClientOpponent>();

        contests_size + lookups_size + std::mem::size_of::<CoreStats>()
    }

    /// Checks if cache needs refresh based on age
    pub fn needs_refresh(&self, max_age_hours: i64) -> bool {
        let age = chrono::Utc::now()
            .fixed_offset()
            .signed_duration_since(self.last_updated)
            .num_hours();

        age > max_age_hours
    }
}

impl Default for CoreStats {
    fn default() -> Self {
        Self {
            total_contests: 0,
            total_wins: 0,
            total_losses: 0,
            win_rate: 0.0,
            average_placement: 0.0,
            best_placement: 0,
            worst_placement: 0,
            current_streak: 0,
            longest_streak: 0,
            skill_rating: 1000.0,
            total_points: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = ClientAnalyticsCache::new("player/test".to_string());
        assert_eq!(cache.player_id, "player/test");
        assert_eq!(cache.contests.len(), 0);
        assert_eq!(cache.core_stats.total_contests, 0);
    }

    #[test]
    fn test_core_stats_computation() {
        let mut cache = ClientAnalyticsCache::new("player/test".to_string());

        // Add a test contest
        let contest = ClientContest {
            id: "contest/test".to_string(),
            name: "Test Contest".to_string(),
            start: chrono::Utc::now().fixed_offset(),
            end: chrono::Utc::now().fixed_offset(),
            game: ClientGame {
                id: "game/test".to_string(),
                name: "Test Game".to_string(),
                year_published: Some(2020),
            },
            venue: ClientVenue {
                id: "venue/test".to_string(),
                name: "Test Venue".to_string(),
                display_name: None,
                city: None,
                state: None,
            },
            participants: vec![ClientParticipant {
                player_id: "player/test".to_string(),
                handle: "testuser".to_string(),
                firstname: Some("Test".to_string()),
                lastname: Some("User".to_string()),
                place: 1,
                result: "won".to_string(),
            }],
            my_result: ClientResult {
                place: 1,
                result: "won".to_string(),
                points: None,
            },
        };

        cache.contests.push(contest);
        cache.compute_core_stats();

        assert_eq!(cache.core_stats.total_contests, 1);
        assert_eq!(cache.core_stats.total_wins, 1);
        assert_eq!(cache.core_stats.win_rate, 100.0);
        assert_eq!(cache.core_stats.best_placement, 1);
    }
}
