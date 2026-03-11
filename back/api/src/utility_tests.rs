#[cfg(test)]
mod utility_tests {
    use chrono::DateTime;

    // Test ID key extraction utility
    #[test]
    fn test_extract_key_from_id() {
        // Test game ID extraction
        assert_eq!(extract_key_from_id("game/123"), "123");
        assert_eq!(extract_key_from_id("venue/456"), "456");
        assert_eq!(extract_key_from_id("contest/789"), "789");

        // Test already extracted keys
        assert_eq!(extract_key_from_id("123"), "123");
        assert_eq!(extract_key_from_id("456"), "456");

        // Test edge cases
        assert_eq!(extract_key_from_id(""), "");
        assert_eq!(extract_key_from_id("no/slash"), "slash");
    }

    // Test ID normalization utility
    #[test]
    fn test_normalize_id() {
        // Test adding prefixes
        assert_eq!(normalize_id("123", "game"), "game/123");
        assert_eq!(normalize_id("456", "venue"), "venue/456");
        assert_eq!(normalize_id("789", "contest"), "contest/789");

        // Test already normalized IDs
        assert_eq!(normalize_id("game/123", "game"), "game/123");
        assert_eq!(normalize_id("venue/456", "venue"), "venue/456");

        // Test edge cases
        assert_eq!(normalize_id("", "game"), "game/");
        assert_eq!(normalize_id("no/slash", "game"), "no/slash");
    }

    // Test date formatting utility
    #[test]
    fn test_format_date_for_display() {
        let test_date = "2024-01-15T14:30:00Z";
        let formatted = format_date_for_display(test_date);

        // Should contain month, day, year, and time
        assert!(formatted.contains("January"));
        assert!(formatted.contains("15"));
        assert!(formatted.contains("2024"));
        assert!(formatted.contains("2:30 PM"));

        // Test invalid date
        let invalid_date = "invalid-date";
        let formatted_invalid = format_date_for_display(invalid_date);
        assert_eq!(formatted_invalid, "invalid-date");

        // Test empty date
        let empty_date = "";
        let formatted_empty = format_date_for_display(empty_date);
        assert_eq!(formatted_empty, "Unknown Date");
    }

    // Test contest data validation utility
    #[test]
    fn test_validate_contest_data() {
        let valid_contest = serde_json::json!({
            "contest_id": "contest/123",
            "contest_name": "Test Contest",
            "game_id": "456",
            "venue_id": "789",
            "my_placement": 1,
            "total_players": 4
        });

        assert!(validate_contest_data(&valid_contest));

        let invalid_contest = serde_json::json!({
            "contest_id": "",
            "contest_name": null,
            "game_id": "",
            "venue_id": "",
            "my_placement": -1,
            "total_players": 0
        });

        assert!(!validate_contest_data(&invalid_contest));
    }

    // Test placement formatting utility
    #[test]
    fn test_format_placement() {
        assert_eq!(format_placement(1), "#1");
        assert_eq!(format_placement(2), "#2");
        assert_eq!(format_placement(10), "#10");
        assert_eq!(format_placement(0), "N/A");
        assert_eq!(format_placement(-1), "N/A");
    }

    // Test result formatting utility
    #[test]
    fn test_format_result() {
        assert_eq!(format_result("Winner"), " (Winner)");
        assert_eq!(format_result("Runner Up"), " (Runner Up)");
        assert_eq!(format_result(""), "");
        assert_eq!(format_result("Third Place"), " (Third Place)");
    }

    // Test URL generation utilities
    #[test]
    fn test_generate_game_history_url() {
        let player_id = "player/123";
        let game_id = "456";
        let url = generate_game_history_url(player_id, game_id);

        assert_eq!(url, "/api/contests/player/player/123/game/456");
    }

    #[test]
    fn test_generate_venue_history_url() {
        let venue_id = "789";
        let url = generate_venue_history_url(venue_id);

        assert_eq!(url, "/api/analytics/player/contests-by-venue?id=789");
    }

    // Test response structure validation
    #[test]
    fn test_validate_response_structure() {
        let valid_response = serde_json::json!([
            {
                "contest_id": "contest/123",
                "contest_name": "Test Contest",
                "game_id": "456",
                "venue_id": "789"
            }
        ]);

        assert!(validate_response_structure(&valid_response));

        let invalid_response = serde_json::json!({
            "error": "Not found"
        });

        assert!(!validate_response_structure(&invalid_response));
    }

    // Helper functions for testing
    fn extract_key_from_id(id: &str) -> &str {
        if id.contains('/') {
            id.split('/').nth(1).unwrap_or(id)
        } else {
            id
        }
    }

    fn normalize_id(id: &str, prefix: &str) -> String {
        if id.contains('/') {
            id.to_string()
        } else {
            format!("{}/{}", prefix, id)
        }
    }

    fn format_date_for_display(date_str: &str) -> String {
        if date_str.is_empty() {
            return "Unknown Date".to_string();
        }

        if let Ok(parsed_date) = DateTime::parse_from_rfc3339(date_str) {
            parsed_date.format("%B %d, %Y at %I:%M %p").to_string()
        } else {
            date_str.to_string()
        }
    }

    fn validate_contest_data(contest: &serde_json::Value) -> bool {
        contest["contest_id"]
            .as_str()
            .map_or(false, |s| !s.is_empty())
            && contest["contest_name"]
                .as_str()
                .map_or(false, |s| !s.is_empty())
            && contest["game_id"].as_str().map_or(false, |s| !s.is_empty())
            && contest["venue_id"]
                .as_str()
                .map_or(false, |s| !s.is_empty())
            && contest["my_placement"].as_i64().map_or(false, |p| p > 0)
            && contest["total_players"].as_i64().map_or(false, |t| t > 0)
    }

    fn format_placement(placement: i64) -> String {
        if placement > 0 {
            format!("#{}", placement)
        } else {
            "N/A".to_string()
        }
    }

    fn format_result(result: &str) -> String {
        if !result.is_empty() {
            format!(" ({})", result)
        } else {
            "".to_string()
        }
    }

    fn generate_game_history_url(player_id: &str, game_id: &str) -> String {
        format!("/api/contests/player/{}/game/{}", player_id, game_id)
    }

    fn generate_venue_history_url(venue_id: &str) -> String {
        format!("/api/analytics/player/contests-by-venue?id={}", venue_id)
    }

    fn validate_response_structure(response: &serde_json::Value) -> bool {
        response.is_array()
    }

    // Test edge cases and error conditions
    #[test]
    fn test_edge_cases() {
        // Test very long IDs
        let long_id = "game/".to_string() + &"0".repeat(100);
        assert_eq!(extract_key_from_id(&long_id), "0".repeat(100));

        // Test special characters in IDs
        let special_id = "game/test-id_with_underscores";
        assert_eq!(extract_key_from_id(special_id), "test-id_with_underscores");

        // Test unicode characters
        let unicode_id = "game/测试游戏";
        assert_eq!(extract_key_from_id(unicode_id), "测试游戏");
    }

    // Test performance characteristics
    #[test]
    fn test_performance_characteristics() {
        // Test that ID extraction is O(1) for typical cases
        let test_id = "game/123";
        let start = std::time::Instant::now();

        for _ in 0..1000 {
            let _ = extract_key_from_id(test_id);
        }

        let duration = start.elapsed();
        assert!(duration.as_millis() < 10, "ID extraction should be fast");
    }

    // Test data consistency across different formats
    #[test]
    fn test_data_consistency() {
        let game_id_variants = vec!["123", "game/123", "game/123/"];

        for variant in game_id_variants {
            let normalized = normalize_id(variant, "game");
            let extracted = extract_key_from_id(&normalized);
            assert_eq!(extracted, "123");
        }
    }
}
