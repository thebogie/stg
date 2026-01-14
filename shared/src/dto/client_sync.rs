use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

/// Request for client-side analytics data sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSyncRequest {
    pub player_id: String,
    /// Last contest ID the client has (for delta sync)
    pub last_contest_id: Option<String>,
    /// Last sync timestamp (for delta sync)
    pub last_sync: Option<DateTime<FixedOffset>>,
    /// Whether to include full data or just deltas
    pub full_sync: bool,
    /// Maximum number of contests to return (for pagination)
    pub limit: Option<usize>,
    /// Whether to include related data (games, venues, players)
    pub include_related: bool,
}

/// Response containing all data needed for client-side analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSyncResponse {
    pub player_id: String,
    pub sync_timestamp: DateTime<FixedOffset>,
    pub data_version: String,

    /// Contest data (flattened for client processing)
    pub contests: Vec<ClientContestDto>,

    /// Related game data
    pub games: Vec<ClientGameDto>,

    /// Related venue data
    pub venues: Vec<ClientVenueDto>,

    /// Related player data (opponents)
    pub players: Vec<ClientPlayerDto>,

    /// Sync metadata
    pub sync_metadata: ClientSyncMetadata,
}

/// Flattened contest data for client processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientContestDto {
    pub id: String,
    pub name: String,
    pub start: DateTime<FixedOffset>,
    pub end: DateTime<FixedOffset>,
    pub game_id: String,
    pub game_name: String,
    pub venue_id: String,
    pub venue_name: String,
    pub venue_display_name: Option<String>,
    pub participants: Vec<ClientParticipantDto>,
    pub my_result: ClientResultDto,
}

/// Participant data for client processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientParticipantDto {
    pub player_id: String,
    pub handle: String,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub place: i32,
    pub result: String, // "won", "lost", "tied"
}

/// User's result in a contest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientResultDto {
    pub place: i32,
    pub result: String,
    pub points: Option<i32>,
}

/// Simplified game data for client storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientGameDto {
    pub id: String,
    pub name: String,
    pub year_published: Option<i32>,
    pub description: Option<String>,
    pub source: String, // "bgg", "manual", etc.
}

/// Simplified venue data for client storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientVenueDto {
    pub id: String,
    pub name: String,
    pub display_name: Option<String>,
    pub formatted_address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub source: String, // "google", "database", etc.
}

/// Simplified player data for client storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientPlayerDto {
    pub id: String,
    pub handle: String,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub email: Option<String>,
    pub last_seen: DateTime<FixedOffset>,
}

/// Metadata about the sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSyncMetadata {
    pub total_contests: usize,
    pub contests_returned: usize,
    pub is_delta: bool,
    pub data_size_bytes: usize,
    pub compression_ratio: Option<f64>,
    pub next_sync_recommended: DateTime<FixedOffset>,
}

/// Request for specific analytics data (for real-time queries)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientAnalyticsRequest {
    pub player_id: String,
    pub query: ClientAnalyticsQuery,
    /// Whether to use cached data or force fresh calculation
    pub use_cache: bool,
}

/// Analytics query parameters for client-side processing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientAnalyticsQuery {
    pub date_range: Option<ClientDateRange>,
    pub games: Option<Vec<String>>,
    pub venues: Option<Vec<String>>,
    pub opponents: Option<Vec<String>>,
    pub min_players: Option<i32>,
    pub max_players: Option<i32>,
    pub result_filter: Option<Vec<String>>, // ["won", "lost"]
    pub placement_range: Option<ClientPlacementRange>,
}

/// Date range for filtering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientDateRange {
    pub start: DateTime<FixedOffset>,
    pub end: DateTime<FixedOffset>,
}

/// Placement range for filtering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientPlacementRange {
    pub min: i32,
    pub max: i32,
}

/// Response for client-side analytics queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientAnalyticsResponse {
    pub player_id: String,
    pub query: ClientAnalyticsQuery,
    pub computed_at: DateTime<FixedOffset>,

    /// Filtered contest data
    pub contests: Vec<ClientContestDto>,

    /// Computed statistics
    pub stats: ClientStatsDto,

    /// Game performance data
    pub game_performance: Vec<ClientGamePerformanceDto>,

    /// Opponent performance data
    pub opponent_performance: Vec<ClientOpponentPerformanceDto>,

    /// Performance trends
    pub trends: Vec<ClientTrendDto>,
}

/// Computed statistics for client display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientStatsDto {
    pub total_contests: i32,
    pub total_wins: i32,
    pub total_losses: i32,
    pub win_rate: f64,
    pub average_placement: f64,
    pub best_placement: i32,
    pub worst_placement: i32,
    pub current_streak: i32,
    pub longest_streak: i32,
}

/// Game performance data for client display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientGamePerformanceDto {
    pub game: ClientGameDto,
    pub total_plays: i32,
    pub wins: i32,
    pub losses: i32,
    pub win_rate: f64,
    pub average_placement: f64,
    pub best_placement: i32,
    pub worst_placement: i32,
    pub last_played: DateTime<FixedOffset>,
    pub days_since_last_play: i64,
    pub favorite_venue: Option<ClientVenueDto>,
}

/// Opponent performance data for client display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientOpponentPerformanceDto {
    pub opponent: ClientPlayerDto,
    pub head_to_head: ClientHeadToHeadDto,
}

/// Head-to-head statistics for client display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientHeadToHeadDto {
    pub total_contests: i32,
    pub my_wins: i32,
    pub opponent_wins: i32,
    pub my_win_rate: f64,
    pub contest_history: Vec<ClientContestDto>,
}

/// Performance trend data for client display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientTrendDto {
    pub period: String, // "2024-01", "2024-Q1", "2024"
    pub contests_played: i32,
    pub wins: i32,
    pub win_rate: f64,
    pub average_placement: f64,
}

/// Request for data validation (client can send data hash to verify integrity)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientDataValidationRequest {
    pub player_id: String,
    pub data_hash: String,
    pub data_version: String,
    pub contest_count: usize,
}

/// Response for data validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientDataValidationResponse {
    pub player_id: String,
    pub is_valid: bool,
    pub server_hash: String,
    pub data_version: String,
    pub contest_count: usize,
    pub validation_message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_sync_request_serialization() {
        let request = ClientSyncRequest {
            player_id: "player/test".to_string(),
            last_contest_id: Some("contest/test".to_string()),
            last_sync: Some(chrono::Utc::now().fixed_offset()),
            full_sync: false,
            limit: Some(100),
            include_related: true,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ClientSyncRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.player_id, deserialized.player_id);
        assert_eq!(request.full_sync, deserialized.full_sync);
        assert_eq!(request.limit, deserialized.limit);
    }

    #[test]
    fn test_client_analytics_query_serialization() {
        let query = ClientAnalyticsQuery {
            date_range: Some(ClientDateRange {
                start: chrono::Utc::now().fixed_offset(),
                end: chrono::Utc::now().fixed_offset(),
            }),
            games: Some(vec!["game/test1".to_string(), "game/test2".to_string()]),
            venues: None,
            opponents: None,
            min_players: Some(2),
            max_players: Some(6),
            result_filter: Some(vec!["won".to_string(), "lost".to_string()]),
            placement_range: Some(ClientPlacementRange { min: 1, max: 3 }),
        };

        let json = serde_json::to_string(&query).unwrap();
        let deserialized: ClientAnalyticsQuery = serde_json::from_str(&json).unwrap();

        assert_eq!(query.games, deserialized.games);
        assert_eq!(query.min_players, deserialized.min_players);
        assert_eq!(query.max_players, deserialized.max_players);
    }
}
