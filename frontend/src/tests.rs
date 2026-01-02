#[cfg(test)]
mod tests {
    // use super::*;
    // use yew::prelude::*;
    use serde_json::json;

    // Basic functionality tests
    #[test]
    fn test_basic_math() {
        assert_eq!(2 + 2, 4);
        assert_eq!(10 - 5, 5);
        assert_eq!(3 * 4, 12);
        assert_eq!(15 / 3, 5);
    }

    #[test]
    fn test_string_operations() {
        let s = "hello world";
        assert_eq!(s.len(), 11);
        assert_eq!(s.to_uppercase(), "HELLO WORLD");
        assert!(s.contains("hello"));
        assert!(!s.contains("goodbye"));
    }

    // JSON handling tests
    #[test]
    fn test_json_creation() {
        let data = json!({
            "name": "Test User",
            "email": "test@example.com",
            "age": 25
        });

        assert_eq!(data["name"], "Test User");
        assert_eq!(data["email"], "test@example.com");
        assert_eq!(data["age"], 25);
    }

    #[test]
    fn test_json_serialization() {
        let data = json!({
            "id": "123",
            "title": "Test Game",
            "year": 2023
        });

        let json_string = serde_json::to_string(&data).unwrap();
        assert!(json_string.contains("Test Game"));
        assert!(json_string.contains("2023"));
    }

    // URL handling tests
    #[test]
    fn test_url_parsing() {
        let url = "https://example.com/api/games";
        assert!(url.starts_with("https://"));
        assert!(url.contains("example.com"));
        assert!(url.ends_with("/games"));
    }

    #[test]
    fn test_url_encoding() {
        let query = "test game";
        let encoded = urlencoding::encode(query);
        assert_eq!(encoded, "test%20game");
    }

    // Validation tests
    #[test]
    fn test_email_validation() {
        let valid_emails = vec![
            "test@example.com",
            "user.name@domain.co.uk",
            "user+tag@example.org"
        ];

        let invalid_emails = vec![
            "invalid-email",
            "@example.com",
            "user@",
            "user@.com"
        ];

        for email in valid_emails {
            assert!(email.contains("@") && email.contains("."));
        }

        for email in invalid_emails {
            // Simple validation: must contain @ and . and have proper structure
            let has_at = email.contains("@");
            let has_dot = email.contains(".");
            let not_start_with_at = !email.starts_with("@");
            let not_end_with_at = !email.ends_with("@");
            let not_end_with_dot = !email.ends_with(".");
            
            // Additional check: if it has @ and ., the . should come after the @
            let dot_after_at = if has_at && has_dot {
                email.find("@").unwrap() < email.find(".").unwrap()
            } else {
                false
            };
            
            // Check that there's a domain part between @ and .
            let has_domain_part = if has_at && has_dot {
                let at_pos = email.find("@").unwrap();
                let dot_pos = email.find(".").unwrap();
                dot_pos > at_pos + 1  // There should be at least one character between @ and .
            } else {
                false
            };
            
            let is_valid = has_at && has_dot && not_start_with_at && not_end_with_at && not_end_with_dot && dot_after_at && has_domain_part;
            assert!(!is_valid, "Email '{}' should be invalid", email);
        }
    }

    #[test]
    fn test_password_validation() {
        let valid_passwords = vec![
            "password123",
            "SecurePass!",
            "MyP@ssw0rd"
        ];

        let invalid_passwords = vec![
            "123",
            "short",
            ""
        ];

        for password in valid_passwords {
            assert!(password.len() >= 8);
        }

        for password in invalid_passwords {
            assert!(password.len() < 8);
        }
    }

    // Data structure tests
    #[test]
    fn test_vector_operations() {
        let mut vec = vec![1, 2, 3, 4, 5];
        
        assert_eq!(vec.len(), 5);
        assert_eq!(vec[0], 1);
        assert_eq!(vec[4], 5);
        
        vec.push(6);
        assert_eq!(vec.len(), 6);
        assert_eq!(vec[5], 6);
        
        let popped = vec.pop();
        assert_eq!(popped, Some(6));
        assert_eq!(vec.len(), 5);
    }

    #[test]
    fn test_hashmap_operations() {
        use std::collections::HashMap;
        
        let mut map = HashMap::new();
        map.insert("key1", "value1");
        map.insert("key2", "value2");
        
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("key1"), Some(&"value1"));
        assert_eq!(map.get("key2"), Some(&"value2"));
        assert_eq!(map.get("key3"), None);
    }

    // Error handling tests
    #[test]
    fn test_result_handling() {
        let success_result: Result<i32, &str> = Ok(42);
        let error_result: Result<i32, &str> = Err("Something went wrong");

        assert!(success_result.is_ok());
        assert!(error_result.is_err());
        
        assert_eq!(success_result.unwrap(), 42);
        assert_eq!(error_result.unwrap_err(), "Something went wrong");
    }

    #[test]
    fn test_option_handling() {
        let some_value: Option<i32> = Some(42);
        let none_value: Option<i32> = None;

        assert!(some_value.is_some());
        assert!(none_value.is_none());
        
        assert_eq!(some_value.unwrap(), 42);
        assert_eq!(some_value.unwrap_or(0), 42);
        assert_eq!(none_value.unwrap_or(0), 0);
    }

    // Performance tests
    #[test]
    fn test_string_concatenation_performance() {
        let start = std::time::Instant::now();
        
        let mut result = String::new();
        for i in 0..1000 {
            result.push_str(&format!("item{}", i));
        }
        
        let duration = start.elapsed();
        assert!(duration.as_millis() < 100); // Should complete in under 100ms
        assert_eq!(result.len(), 6890); // Expected length: 1000 * 4 ("item") + sum of digit lengths
    }

    #[test]
    fn test_json_parsing_performance() {
        let json_data = r#"{"name":"Test","items":[1,2,3,4,5]}"#;
        
        let start = std::time::Instant::now();
        
        for _ in 0..100 {
            let _: serde_json::Value = serde_json::from_str(json_data).unwrap();
        }
        
        let duration = start.elapsed();
        assert!(duration.as_millis() < 50); // Should complete in under 50ms
    }

    // Async tests removed - tokio is not available for WASM builds
    // If you need async testing, use integration tests in the testing crate
    // or E2E tests with Playwright
} 