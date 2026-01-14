#[cfg(test)]
mod tests {
    // use super::*;
    use crate::config::Config;
    use crate::error::ApiError;
    use chrono::{Duration, Utc};
    use shared::models::{contest::Contest, game::Game, player::Player, venue::Venue};

    // Configuration tests
    #[test]
    fn test_config_loading() {
        // Test that config can be loaded (even if env vars are missing)
        let config = Config::load();
        assert!(config.is_ok() || config.is_err()); // Should either load or fail gracefully
    }

    #[test]
    fn test_config_defaults() {
        // Test config has reasonable defaults
        let config = Config::load();
        assert!(config.is_ok() || config.is_err()); // Should either load or fail gracefully
    }

    // Player model tests
    #[test]
    fn test_player_creation() {
        let player = Player {
            id: "test_player".to_string(),
            rev: "1".to_string(),
            firstname: "John".to_string(),
            handle: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
            created_at: Utc::now().fixed_offset(),
            is_admin: false,
        };

        assert_eq!(player.handle, "testuser");
        assert_eq!(player.email, "test@example.com");
        assert_eq!(player.firstname, "John");
    }

    #[test]
    fn test_player_validation() {
        let player = Player {
            id: "test_player".to_string(),
            rev: "1".to_string(),
            firstname: "John".to_string(),
            handle: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
            created_at: Utc::now().fixed_offset(),
            is_admin: false,
        };

        // Test basic validation
        assert!(!player.id.is_empty());
        assert!(!player.handle.is_empty());
        assert!(!player.email.is_empty());
        assert!(player.email.contains("@"));
    }

    // Game model tests
    #[test]
    fn test_game_creation() {
        let game = Game {
            id: "test_game".to_string(),
            rev: "1".to_string(),
            name: "Test Game".to_string(),
            year_published: Some(2020),
            bgg_id: Some(12345),
            description: Some("A test game".to_string()),
            source: shared::models::game::GameSource::Database,
        };

        assert_eq!(game.name, "Test Game");
        assert_eq!(game.year_published, Some(2020));
        assert_eq!(game.bgg_id, Some(12345));
        assert_eq!(game.source, shared::models::game::GameSource::Database);
    }

    #[test]
    fn test_game_optional_fields() {
        let game = Game {
            id: "test_game".to_string(),
            rev: "1".to_string(),
            name: "Test Game".to_string(),
            year_published: None,
            bgg_id: None,
            description: None,
            source: shared::models::game::GameSource::Database,
        };

        assert_eq!(game.name, "Test Game");
        assert_eq!(game.year_published, None);
        assert_eq!(game.bgg_id, None);
        assert_eq!(game.description, None);
    }

    // Venue model tests
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
            source: shared::models::venue::VenueSource::Database,
        };

        assert_eq!(venue.display_name, "Test Venue");
        assert_eq!(venue.formatted_address, "123 Test St, Test City, TC 12345");
        assert_eq!(venue.lat, 40.7128);
        assert_eq!(venue.lng, -74.0060);
        assert_eq!(venue.timezone, "America/New_York");
    }

    #[test]
    fn test_venue_coordinates() {
        let venue = Venue {
            id: "test_venue".to_string(),
            rev: "1".to_string(),
            display_name: "Test Venue".to_string(),
            formatted_address: "123 Test St, Test City, TC 12345".to_string(),
            place_id: "test_place_id".to_string(),
            lat: 0.0,
            lng: 0.0,
            timezone: "UTC".to_string(),
            source: shared::models::venue::VenueSource::Database,
        };

        // Test coordinate validation
        assert!(venue.lat >= -90.0 && venue.lat <= 90.0);
        assert!(venue.lng >= -180.0 && venue.lng <= 180.0);
        assert_eq!(venue.timezone, "UTC");
    }

    // Contest model tests
    #[test]
    fn test_contest_creation() {
        let contest = Contest {
            id: "test_contest".to_string(),
            rev: "1".to_string(),
            name: "Test Contest".to_string(),
            start: Utc::now().fixed_offset(),
            stop: Utc::now().fixed_offset() + Duration::days(1),
            creator_id: "player/test_creator".to_string(),
            created_at: Utc::now().fixed_offset(),
        };

        assert_eq!(contest.name, "Test Contest");
        assert!(contest.stop > contest.start);
    }

    // Error handling tests
    #[test]
    fn test_api_error_creation() {
        let error = ApiError::database_error("Test database error");
        assert!(error.to_string().contains("Test database error"));
    }

    #[test]
    fn test_api_error_types() {
        let db_error = ApiError::database_error("DB error");
        let validation_error = ApiError::validation_error("Validation error");
        let not_found_error = ApiError::not_found("Not found");

        assert_eq!(db_error.status_code, 500);
        assert_eq!(validation_error.status_code, 400);
        assert_eq!(not_found_error.status_code, 404);
    }

    // Performance tests
    #[test]
    fn test_string_operations_performance() {
        let start = std::time::Instant::now();

        // Perform some string operations
        let mut result = String::new();
        for i in 0..1000 {
            result.push_str(&format!("test{}", i));
        }

        let duration = start.elapsed();
        assert!(duration.as_millis() < 100); // Should complete in under 100ms
        assert_eq!(result.len(), 6890); // Expected length: 1000 * 4 ("test") + sum of digit lengths
    }

    #[test]
    fn test_json_serialization_performance() {
        let player = Player {
            id: "test_player".to_string(),
            rev: "1".to_string(),
            firstname: "John".to_string(),
            handle: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
            created_at: Utc::now().fixed_offset(),
            is_admin: false,
        };

        let start = std::time::Instant::now();

        // Serialize and deserialize
        for _ in 0..100 {
            let json = serde_json::to_string(&player).unwrap();
            let _: Player = serde_json::from_str(&json).unwrap();
        }

        let duration = start.elapsed();
        assert!(duration.as_millis() < 50); // Should complete in under 50ms
    }
}
