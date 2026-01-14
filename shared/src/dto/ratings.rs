use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type", content = "id")]
pub enum RatingScope {
    Global,
    Game(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRatingDto {
    pub player_id: String,
    pub scope: RatingScope,
    pub rating: f64,
    pub rd: f64,
    pub volatility: f64,
    pub games_played: i32,
    pub last_period_end: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRatingHistoryPointDto {
    pub player_id: String,
    pub scope: RatingScope,
    pub period_end: String,
    pub rating: f64,
    pub rd: f64,
    pub volatility: f64,
    pub period_games: i32,
    pub wins: i32,
    pub losses: i32,
    pub draws: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingLeaderboardEntryDto {
    pub player_id: String,
    pub handle: Option<String>,
    pub rating: f64,
    pub rd: f64,
    pub games_played: i32,
    pub last_active: Option<String>,
    pub contest_id: Option<String>,
}
