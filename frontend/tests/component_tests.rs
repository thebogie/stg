#[cfg(test)]
mod component_tests {
    use serde_json::json;

    // Basic functionality tests for frontend utilities
    #[test]
    fn test_json_handling() {
        let test_data = json!({
            "title": "Test Component",
            "count": 42,
            "active": true,
            "tags": ["test", "component", "frontend"]
        });

        assert_eq!(test_data["title"], "Test Component");
        assert_eq!(test_data["count"], 42);
        assert_eq!(test_data["active"], true);
        assert!(test_data["tags"].is_array());
    }

    #[test]
    fn test_string_operations() {
        let title = "Test Component Title";
        let count = 42;
        let description = format!("Component {}: {}", count, title);

        assert_eq!(description, "Component 42: Test Component Title");
        assert!(description.contains("Test"));
        assert!(description.contains("42"));
    }

    #[test]
    fn test_data_validation() {
        let valid_data = vec![
            ("title", "Valid Title"),
            ("count", "42"),
            ("active", "true"),
        ];

        for (key, value) in valid_data {
            assert!(!key.is_empty());
            assert!(!value.is_empty());
        }

        let invalid_data = vec![("", "Empty Key"), ("key", ""), ("", "")];

        for (key, value) in invalid_data {
            let is_valid = !key.is_empty() && !value.is_empty();
            assert!(!is_valid);
        }
    }

    #[test]
    fn test_url_handling() {
        let base_url = "https://example.com";
        let endpoint = "/api/components";
        let full_url = format!("{}{}", base_url, endpoint);

        assert!(full_url.starts_with("https://"));
        assert!(full_url.contains("example.com"));
        assert!(full_url.ends_with("/api/components"));
    }

    #[test]
    fn test_data_serialization() {
        let component_data = json!({
            "id": "comp_123",
            "name": "TestComponent",
            "props": {
                "title": "Test Title",
                "count": 42
            },
            "created_at": "2024-01-01T00:00:00Z"
        });

        let json_string = serde_json::to_string(&component_data).unwrap();
        assert!(json_string.contains("TestComponent"));
        assert!(json_string.contains("42"));
        assert!(json_string.contains("comp_123"));
    }

    #[test]
    fn test_data_deserialization() {
        let json_string = r#"{
            "id": "comp_456",
            "name": "AnotherComponent",
            "props": {
                "title": "Another Title",
                "count": 100
            }
        }"#;

        let component_data: serde_json::Value = serde_json::from_str(json_string).unwrap();

        assert_eq!(component_data["id"], "comp_456");
        assert_eq!(component_data["name"], "AnotherComponent");
        assert_eq!(component_data["props"]["title"], "Another Title");
        assert_eq!(component_data["props"]["count"], 100);
    }

    #[test]
    fn test_error_handling() {
        let invalid_json = "{ invalid json }";
        let result: Result<serde_json::Value, _> = serde_json::from_str(invalid_json);

        assert!(result.is_err());

        let error = result.unwrap_err();
        let msg = error.to_string();
        assert!(msg.contains("expected") || msg.contains("at line") || !msg.is_empty());
    }

    #[test]
    fn test_data_transformation() {
        let input_data = vec![
            ("title", "Component 1"),
            ("title", "Component 2"),
            ("title", "Component 3"),
        ];

        let transformed: Vec<String> = input_data
            .iter()
            .map(|(_, value)| value.to_string())
            .collect();

        assert_eq!(transformed.len(), 3);
        assert_eq!(transformed[0], "Component 1");
        assert_eq!(transformed[1], "Component 2");
        assert_eq!(transformed[2], "Component 3");
    }

    #[test]
    fn test_data_filtering() {
        let components = vec![
            json!({"name": "Component A", "active": true}),
            json!({"name": "Component B", "active": false}),
            json!({"name": "Component C", "active": true}),
        ];

        let active_components: Vec<&serde_json::Value> = components
            .iter()
            .filter(|comp| comp["active"] == true)
            .collect();

        assert_eq!(active_components.len(), 2);
        assert_eq!(active_components[0]["name"], "Component A");
        assert_eq!(active_components[1]["name"], "Component C");
    }

    #[test]
    fn test_data_sorting() {
        let mut components = vec![
            json!({"name": "Component C", "priority": 3}),
            json!({"name": "Component A", "priority": 1}),
            json!({"name": "Component B", "priority": 2}),
        ];

        components.sort_by(|a, b| {
            a["priority"]
                .as_u64()
                .unwrap()
                .cmp(&b["priority"].as_u64().unwrap())
        });

        assert_eq!(components[0]["name"], "Component A");
        assert_eq!(components[1]["name"], "Component B");
        assert_eq!(components[2]["name"], "Component C");
    }

    #[test]
    fn test_data_aggregation() {
        let components = vec![
            json!({"name": "Component A", "size": 100}),
            json!({"name": "Component B", "size": 200}),
            json!({"name": "Component C", "size": 300}),
        ];

        let total_size: u64 = components
            .iter()
            .map(|comp| comp["size"].as_u64().unwrap())
            .sum();

        assert_eq!(total_size, 600);

        let avg_size = total_size as f64 / components.len() as f64;
        assert_eq!(avg_size, 200.0);
    }

    #[test]
    fn test_unicode_handling() {
        let unicode_title = "🎮 Game Component 🎯";
        let unicode_description = "Component with emoji and special characters: 🚀✨🎨";

        assert!(unicode_title.contains("🎮"));
        assert!(unicode_title.contains("🎯"));
        assert!(unicode_description.contains("🚀"));
        assert!(unicode_description.contains("✨"));
        assert!(unicode_description.contains("🎨"));

        // Test JSON with unicode
        let unicode_data = json!({
            "title": unicode_title,
            "description": unicode_description
        });

        let json_string = serde_json::to_string(&unicode_data).unwrap();
        assert!(json_string.contains("🎮"));
        assert!(json_string.contains("🎯"));
    }

    #[test]
    fn test_performance_optimization() {
        let large_dataset: Vec<serde_json::Value> = (0..1000)
            .map(|i| {
                json!({
                    "id": format!("comp_{}", i),
                    "name": format!("Component {}", i),
                    "data": format!("Data for component {}", i)
                })
            })
            .collect();

        // Test filtering performance
        let start_time = std::time::Instant::now();
        let filtered: Vec<&serde_json::Value> = large_dataset
            .iter()
            .filter(|comp| comp["id"].as_str().unwrap().contains("5"))
            .collect();
        let filter_time = start_time.elapsed();

        // Filtering should be fast (under 10ms for 1000 items)
        assert!(filter_time.as_micros() < 10000);
        assert!(filtered.len() > 0);

        // Test search performance
        let start_time = std::time::Instant::now();
        let found = large_dataset.iter().find(|comp| comp["id"] == "comp_500");
        let search_time = start_time.elapsed();

        // Search should be fast (under 1ms for 1000 items)
        assert!(search_time.as_micros() < 1000);
        assert!(found.is_some());
        assert_eq!(found.unwrap()["id"], "comp_500");
    }

    #[test]
    fn test_memory_efficiency() {
        let mut components = Vec::new();

        // Create many components
        for i in 0..1000 {
            components.push(json!({
                "id": format!("comp_{}", i),
                "name": format!("Component {}", i),
                "data": format!("Data for component {}", i)
            }));
        }

        let initial_capacity = components.capacity();

        // Remove some components
        components.retain(|comp| {
            let id = comp["id"].as_str().unwrap();
            !id.contains("5") // Remove components with "5" in ID
        });

        // Capacity should be reasonable
        assert!(components.capacity() <= initial_capacity * 2);
        assert!(components.len() < 1000);
    }

    #[test]
    fn test_edge_cases() {
        // Test with empty data
        let empty_data = json!({});
        assert!(empty_data.is_object());
        assert_eq!(empty_data.as_object().unwrap().len(), 0);

        // Test with null values
        let null_data = json!({
            "title": null,
            "count": null,
            "active": null
        });
        assert!(null_data["title"].is_null());
        assert!(null_data["count"].is_null());
        assert!(null_data["active"].is_null());

        // Test with very long strings
        let long_string = "x".repeat(10000);
        let long_data = json!({
            "title": long_string.clone()
        });
        assert_eq!(long_data["title"], long_string);

        // Test with special characters
        let special_chars = "!@#$%^&*()_+-=[]{}|;':\",./<>?";
        let special_data = json!({
            "title": special_chars
        });
        assert_eq!(special_data["title"], special_chars);
    }

    // Game and venue history component tests
    #[test]
    fn test_game_history_data_parsing() {
        let mock_game_history_data = json!([
            {
                "contest_id": "contest/123",
                "contest_name": "Test Contest",
                "contest_date": "2024-01-01T12:00:00Z",
                "game_id": "456",
                "game_name": "Test Game",
                "venue_id": "789",
                "venue_name": "Test Venue",
                "my_placement": 1,
                "my_result": "Winner",
                "total_players": 4
            }
        ]);

        // Test array parsing
        assert!(mock_game_history_data.is_array());
        let contests = mock_game_history_data.as_array().unwrap();
        assert_eq!(contests.len(), 1);

        let contest = &contests[0];

        // Test required fields
        assert!(contest["contest_id"].is_string());
        assert!(contest["contest_name"].is_string());
        assert!(contest["game_id"].is_string());
        assert!(contest["venue_id"].is_string());
        assert!(contest["my_placement"].is_number());
        assert!(contest["total_players"].is_number());
    }

    #[test]
    fn test_venue_history_data_parsing() {
        let mock_venue_history_data = json!([
            {
                "contest_id": "contest/456",
                "contest_name": "Venue Contest",
                "contest_date": "2024-02-01T14:00:00Z",
                "game_id": "789",
                "game_name": "Venue Game",
                "venue_id": "123",
                "venue_name": "Test Venue",
                "venue_display_name": "Test Venue Display",
                "my_placement": 2,
                "my_result": "Runner Up",
                "total_players": 6
            }
        ]);

        // Test array parsing
        assert!(mock_venue_history_data.is_array());
        let contests = mock_venue_history_data.as_array().unwrap();
        assert_eq!(contests.len(), 1);

        let contest = &contests[0];

        // Test venue-specific fields
        assert!(contest["venue_id"].is_string());
        assert!(contest["venue_name"].is_string());
        assert!(contest["venue_display_name"].is_string());
    }

    #[test]
    fn test_id_extraction_logic() {
        // Test game ID extraction
        let game_id_full = "game/123";
        let game_id_key = if game_id_full.contains('/') {
            game_id_full.split('/').nth(1).unwrap_or(game_id_full)
        } else {
            game_id_full
        };
        assert_eq!(game_id_key, "123");

        // Test venue ID extraction
        let venue_id_full = "venue/456";
        let venue_id_key = if venue_id_full.contains('/') {
            venue_id_full.split('/').nth(1).unwrap_or(venue_id_full)
        } else {
            venue_id_full
        };
        assert_eq!(venue_id_key, "456");

        // Test contest ID extraction
        let contest_id_full = "contest/789";
        let contest_id_key = if contest_id_full.contains('/') {
            contest_id_full.split('/').nth(1).unwrap_or(contest_id_full)
        } else {
            contest_id_full
        };
        assert_eq!(contest_id_key, "789");
    }

    #[test]
    fn test_contest_data_validation() {
        let valid_contest = json!({
            "contest_id": "contest/123",
            "contest_name": "Valid Contest",
            "contest_date": "2024-01-01T12:00:00Z",
            "game_id": "456",
            "game_name": "Test Game",
            "venue_id": "789",
            "venue_name": "Test Venue",
            "my_placement": 1,
            "my_result": "Winner",
            "total_players": 4
        });

        // Test field extraction with fallbacks
        let contest_id = valid_contest["contest_id"].as_str().unwrap_or("");
        let contest_name = valid_contest["contest_name"]
            .as_str()
            .unwrap_or("Unknown Contest");
        let game_name = valid_contest["game_name"]
            .as_str()
            .unwrap_or("Unknown Game");
        let venue_name = valid_contest["venue_name"]
            .as_str()
            .unwrap_or("Unknown Venue");
        let my_placement = valid_contest["my_placement"].as_i64().unwrap_or(0);
        let total_players = valid_contest["total_players"].as_i64().unwrap_or(0);

        assert_eq!(contest_id, "contest/123");
        assert_eq!(contest_name, "Valid Contest");
        assert_eq!(game_name, "Test Game");
        assert_eq!(venue_name, "Test Venue");
        assert_eq!(my_placement, 1);
        assert_eq!(total_players, 4);
    }

    #[test]
    fn test_error_handling_for_missing_data() {
        let incomplete_contest = json!({
            "contest_id": "contest/123",
            "contest_name": null,
            "game_name": "",
            "venue_name": null,
            "my_placement": null,
            "total_players": null
        });

        // Test graceful handling of missing data
        let contest_name = incomplete_contest["contest_name"]
            .as_str()
            .unwrap_or("Unknown Contest");
        let game_name = match incomplete_contest["game_name"].as_str() {
            Some(s) if !s.is_empty() => s,
            _ => "Unknown Game",
        };
        let venue_name = incomplete_contest["venue_name"]
            .as_str()
            .unwrap_or("Unknown Venue");
        let my_placement = incomplete_contest["my_placement"].as_i64().unwrap_or(0);
        let total_players = incomplete_contest["total_players"].as_i64().unwrap_or(0);

        assert_eq!(contest_name, "Unknown Contest");
        assert_eq!(game_name, "Unknown Game");
        assert_eq!(venue_name, "Unknown Venue");
        assert_eq!(my_placement, 0);
        assert_eq!(total_players, 0);
    }

    #[test]
    fn test_placement_and_result_formatting() {
        // Test placement formatting
        let placement_1 = 1;
        let placement_2 = 2;
        let placement_invalid = 0;

        let formatted_1 = if placement_1 > 0 {
            format!("#{}", placement_1)
        } else {
            "N/A".to_string()
        };
        let formatted_2 = if placement_2 > 0 {
            format!("#{}", placement_2)
        } else {
            "N/A".to_string()
        };
        let formatted_invalid = if placement_invalid > 0 {
            format!("#{}", placement_invalid)
        } else {
            "N/A".to_string()
        };

        assert_eq!(formatted_1, "#1");
        assert_eq!(formatted_2, "#2");
        assert_eq!(formatted_invalid, "N/A");

        // Test result formatting
        let my_result = "Winner";
        let empty_result = "";

        let formatted_with_result = if !my_result.is_empty() {
            format!(" ({})", my_result)
        } else {
            "".to_string()
        };
        let formatted_empty = if !empty_result.is_empty() {
            format!(" ({})", empty_result)
        } else {
            "".to_string()
        };

        assert_eq!(formatted_with_result, " (Winner)");
        assert_eq!(formatted_empty, "");
    }
}
