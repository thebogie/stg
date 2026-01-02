#[cfg(test)]
mod tests {
    // use super::*;

    // Test the normalization logic that's now embedded in the controllers
    fn normalize_id(collection: &str, key_or_id: &str) -> String {
        if key_or_id.contains('/') { 
            key_or_id.to_string() 
        } else { 
            format!("{}/{}", collection, key_or_id) 
        }
    }

    #[test]
    fn test_normalize_id_with_key_only() {
        assert_eq!(normalize_id("player", "123"), "player/123");
        assert_eq!(normalize_id("game", "456"), "game/456");
        assert_eq!(normalize_id("venue", "789"), "venue/789");
        assert_eq!(normalize_id("contest", "abc"), "contest/abc");
    }

    #[test]
    fn test_normalize_id_with_full_id() {
        assert_eq!(normalize_id("player", "player/123"), "player/123");
        assert_eq!(normalize_id("game", "game/456"), "game/456");
        assert_eq!(normalize_id("venue", "venue/789"), "venue/789");
        assert_eq!(normalize_id("contest", "contest/abc"), "contest/abc");
    }

    #[test]
    fn test_normalize_id_with_edge_cases() {
        // Empty key
        assert_eq!(normalize_id("player", ""), "player/");
        
        // Key with special characters
        assert_eq!(normalize_id("player", "user-123_test"), "player/user-123_test");
        
        // Key that looks like a path but isn't a full ID
        assert_eq!(normalize_id("player", "user/123"), "user/123");
    }

    #[test]
    fn test_normalize_id_preserves_existing_format() {
        // Should not double-prefix
        assert_eq!(normalize_id("player", "player/123"), "player/123");
        assert_eq!(normalize_id("game", "game/456"), "game/456");
        
        // Should handle different collection names
        assert_eq!(normalize_id("different", "player/123"), "player/123");
    }
}
