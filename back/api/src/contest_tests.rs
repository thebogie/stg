#[cfg(test)]
mod contest_tests {
    // use super::*;
    use chrono::{Duration, Utc};
    use shared::dto::contest::{ContestDto, OutcomeDto};
    use shared::models::contest::Contest;

    #[test]
    fn test_contest_dto_creation() {
        let contest_dto = ContestDto {
            id: "contest/test".to_string(),
            name: "Test Contest".to_string(),
            start: Utc::now().fixed_offset(),
            stop: Utc::now().fixed_offset() + Duration::hours(2),
            venue: shared::dto::venue::VenueDto {
                id: "venue/test".to_string(),
                display_name: "Test Venue".to_string(),
                formatted_address: "123 Test St".to_string(),
                place_id: "test_place".to_string(),
                lat: 40.7128,
                lng: -74.0060,
                timezone: "America/New_York".to_string(),
                source: shared::models::venue::VenueSource::Database,
            },
            games: vec![],
            outcomes: vec![],
            creator_id: String::new(),
            created_at: None,
        };

        assert_eq!(contest_dto.name, "Test Contest");
        assert!(contest_dto.stop > contest_dto.start);
    }

    #[test]
    fn test_outcome_dto_creation() {
        let outcome = OutcomeDto {
            player_id: "player/test".to_string(),
            place: "1".to_string(),
            result: "won".to_string(),
            email: "test@example.com".to_string(),
            handle: "testplayer".to_string(),
        };

        assert_eq!(outcome.player_id, "player/test");
        assert_eq!(outcome.place, "1");
        assert_eq!(outcome.result, "won");
    }

    #[test]
    fn test_contest_model_creation() {
        let contest = Contest {
            id: "contest/test".to_string(),
            rev: "1".to_string(),
            name: "Test Contest".to_string(),
            start: Utc::now().fixed_offset(),
            stop: Utc::now().fixed_offset() + Duration::hours(2),
            creator_id: "player/test-creator".to_string(),
            created_at: Utc::now().fixed_offset(),
        };

        assert_eq!(contest.name, "Test Contest");
        assert!(contest.stop > contest.start);
    }

    #[test]
    fn test_contest_serialization() {
        let contest = Contest {
            id: "contest/test".to_string(),
            rev: "1".to_string(),
            name: "Test Contest".to_string(),
            start: Utc::now().fixed_offset(),
            stop: Utc::now().fixed_offset() + Duration::hours(2),
            creator_id: "player/test-creator".to_string(),
            created_at: Utc::now().fixed_offset(),
        };

        let json = serde_json::to_string(&contest).unwrap();
        let deserialized: Contest = serde_json::from_str(&json).unwrap();

        assert_eq!(contest.id, deserialized.id);
        assert_eq!(contest.name, deserialized.name);
    }
}

#[cfg(test)]
mod contest_integration_like_tests {
    use chrono::{Duration, FixedOffset, Utc};
    use shared::dto::contest::ContestDto;
    use shared::models::venue::VenueSource;

    #[test]
    fn test_utc_and_timezone_fields_present() {
        // Start/stop should be UTC instants; venue timezone preserved
        let start_utc = Utc::now().fixed_offset();
        let stop_utc = start_utc + Duration::hours(1);
        let contest_dto = ContestDto {
            id: "contest/test2".to_string(),
            name: "Test".to_string(),
            start: start_utc,
            stop: stop_utc,
            venue: shared::dto::venue::VenueDto {
                id: "".to_string(),
                display_name: "Paris Orly Airport".to_string(),
                formatted_address: "94390 Orly, France".to_string(),
                place_id: "pid".to_string(),
                lat: 48.72,
                lng: 2.38,
                timezone: "Europe/Paris".to_string(),
                source: VenueSource::Google,
            },
            games: vec![],
            outcomes: vec![],
            creator_id: String::new(),
            created_at: None,
        };
        assert!(contest_dto.stop > contest_dto.start);
        assert_eq!(contest_dto.venue.timezone, "Europe/Paris");
        // Ensure UTC instant is usable
        let _ = contest_dto
            .start
            .with_timezone(&FixedOffset::east_opt(0).unwrap());
    }
}
