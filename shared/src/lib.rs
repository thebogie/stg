pub mod models {
    pub mod analytics;
    pub mod auth;
    pub mod client_analytics;
    pub mod client_storage;
    pub mod contest;
    pub mod game;
    pub mod player;
    pub mod relations;
    pub mod venue;
}

pub mod dto {
    pub mod analytics;
    pub mod auth;
    pub mod client_sync;
    pub mod common;
    pub mod contest;
    pub mod game;
    pub mod outcome;
    pub mod player;
    pub mod ratings;
    pub mod relations;
    pub mod venue;
}

pub mod error;
pub mod timezone;
pub mod timezone_cache;

// Re-export commonly used items
pub use error::{Result, SharedError};

// Re-export models
pub use models::{
    analytics::{
        Achievement, AchievementCategory, ContestStats, GamePopularity, GameStats, MonthlyContests,
        MonthlyPlays, PlatformStats, PlayerAchievements, PlayerStats, PlayerWinRate, VenueActivity,
        VenueStats,
    },
    auth::{LoginRequest, RegisterRequest, User, UserSession},
    contest::Contest,
    game::Game,
    player::Player,
    relations::{PlayedAt, PlayedWith, ResultedIn},
    venue::Venue,
};

// Re-export DTOs
pub use dto::{
    analytics::{
        AchievementCategoryDto, AchievementDto, ContestStatsDto, ContestStatsRequest,
        GamePopularityDto, GameStatsDto, GameStatsRequest, LeaderboardCategory, LeaderboardEntry,
        LeaderboardRequest, LeaderboardResponse, MonthlyContestsDto, MonthlyPlaysDto,
        PlatformStatsDto, PlayerAchievementsDto, PlayerStatsDto, PlayerStatsRequest,
        PlayerWinRateDto, TimePeriod, VenueActivityDto, VenueStatsDto, VenueStatsRequest,
    },
    auth::UserSessionDto,
    common::{AuthResponse, ErrorResponse, SearchQuery},
    contest::{ContestDto, OutcomeDto},
    game::GameDto,
    outcome::Outcome,
    player::{CreatePlayerRequest, LoginResponse, PlayerDto, PlayerProfileDto, StoredPlayer},
    ratings::{
        PlayerRatingDto, PlayerRatingHistoryPointDto, RatingLeaderboardEntryDto, RatingScope,
    },
    relations::{PlayedAtDto, PlayedWithDto, ResultedInDto},
    venue::VenueDto,
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
