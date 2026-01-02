pub mod models {
    pub mod contest;
    pub mod game;
    pub mod venue;
    pub mod player;
    pub mod auth;
    pub mod relations;
    pub mod analytics;
    pub mod client_analytics;
    pub mod client_storage;
}

pub mod dto {
    pub mod contest;
    pub mod game;
    pub mod venue;
    pub mod player;
    pub mod auth;
    pub mod relations;
    pub mod outcome;
    pub mod common;
    pub mod analytics;
    pub mod client_sync;
    pub mod ratings;
}

pub mod error;
pub mod timezone;
pub mod timezone_cache;

// Re-export commonly used items
pub use error::{SharedError, Result};

// Re-export models
pub use models::{
    player::Player,
    contest::Contest,
    game::Game,
    venue::Venue,
    auth::{User, UserSession, LoginRequest, RegisterRequest},
    relations::{PlayedAt, PlayedWith, ResultedIn},
    analytics::{
        PlayerStats, ContestStats, GameStats, VenueStats, PlatformStats,
        PlayerWinRate, MonthlyPlays, GamePopularity, MonthlyContests,
        VenueActivity, Achievement, AchievementCategory, PlayerAchievements,
    },
};

// Re-export DTOs
pub use dto::{
    player::{PlayerDto, CreatePlayerRequest, LoginResponse, StoredPlayer, PlayerProfileDto},
    contest::{ContestDto, OutcomeDto},
    game::GameDto,
    venue::VenueDto,
    relations::{PlayedAtDto, PlayedWithDto, ResultedInDto},
    auth::UserSessionDto,
    common::{SearchQuery, ErrorResponse, AuthResponse},
    outcome::Outcome,
    analytics::{
        PlayerStatsDto, ContestStatsDto, GameStatsDto, VenueStatsDto, PlatformStatsDto,
        PlayerWinRateDto, MonthlyPlaysDto, GamePopularityDto, MonthlyContestsDto,
        VenueActivityDto, AchievementDto, AchievementCategoryDto, PlayerAchievementsDto,
        PlayerStatsRequest, ContestStatsRequest, GameStatsRequest, VenueStatsRequest,
        LeaderboardRequest, LeaderboardCategory, TimePeriod, LeaderboardEntry, LeaderboardResponse,
    },
    ratings::{
        RatingScope, PlayerRatingDto, PlayerRatingHistoryPointDto, RatingLeaderboardEntryDto,
    },
};

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_player_creation() {
        let player = Player {
            id: "test_player".to_string(),
            rev: "1".to_string(),
            firstname: "John".to_string(),
            handle: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        
        assert_eq!(player.handle, "testuser");
        assert_eq!(player.email, "test@example.com");
    }

    #[test]
    fn test_venue_creation() {
        let venue = Venue {
            id: "test_venue".to_string(),
            rev: "1".to_string(),
            display_name: "Test Venue".to_string(),
            formatted_address: "123 Test St, Test City, TC 12345".to_string(),
            place_id: "test_place_id".to_string(),
            lat: 40.7128,
            lng: -74.0060,
            timezone: "America/New_York".to_string(),
            source: crate::models::venue::VenueSource::Database,
        };
        
        assert_eq!(venue.display_name, "Test Venue");
        assert_eq!(venue.formatted_address, "123 Test St, Test City, TC 12345");
        assert_eq!(venue.timezone, "America/New_York");
    }

    #[test]
    fn test_game_creation() {
        let game = Game {
            id: "test_game".to_string(),
            rev: "1".to_string(),
            name: "Test Game".to_string(),
            year_published: Some(2020),
            bgg_id: Some(12345),
            description: Some("A test game".to_string()),
            source: crate::models::game::GameSource::Database,
        };
        
        assert_eq!(game.name, "Test Game");
        assert_eq!(game.year_published, Some(2020));
        assert_eq!(game.bgg_id, Some(12345));
    }
} 