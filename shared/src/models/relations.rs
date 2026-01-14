use crate::error::{Result, SharedError};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Represents a "played at" relation between a contest and a venue
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlayedAt {
    /// ArangoDB document ID (format: "played_at/{timestamp}")
    #[serde(rename = "_id")]
    pub id: String,

    /// ArangoDB document revision
    #[serde(rename = "_rev")]
    pub rev: String,

    /// Target vertex ID (format: "venue/{timestamp}")
    #[serde(rename = "_to")]
    #[validate(length(min = 1))]
    pub to: String,

    /// Source vertex ID (format: "contest/{timestamp}")
    #[serde(rename = "_from")]
    #[validate(length(min = 1))]
    pub from: String,

    /// Edge label (always "PLAYED_AT")
    #[serde(rename = "_label")]
    #[validate(length(min = 1))]
    pub label: String,
}

/// Represents a "played with" relation between a contest and a game
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlayedWith {
    /// ArangoDB document ID (format: "played_with/{timestamp}")
    #[serde(rename = "_id")]
    pub id: String,

    /// ArangoDB document revision
    #[serde(rename = "_rev")]
    pub rev: String,

    /// Target vertex ID (format: "game/{timestamp}")
    #[serde(rename = "_to")]
    #[validate(length(min = 1))]
    pub to: String,

    /// Source vertex ID (format: "contest/{timestamp}")
    #[serde(rename = "_from")]
    #[validate(length(min = 1))]
    pub from: String,

    /// Edge label (always "PLAYED_WITH")
    #[serde(rename = "_label")]
    #[validate(length(min = 1))]
    pub label: String,
}

/// Represents a "resulted in" relation from a contest to a player
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ResultedIn {
    /// ArangoDB document ID (format: "resulted_in/{timestamp}")
    #[serde(rename = "_id")]
    pub id: String,

    /// ArangoDB document revision
    #[serde(rename = "_rev")]
    pub rev: String,

    /// Target vertex ID (format: "player/{timestamp}")
    #[serde(rename = "_to")]
    #[validate(length(min = 1))]
    pub to: String,

    /// Source vertex ID (format: "contest/{timestamp}")
    #[serde(rename = "_from")]
    #[validate(length(min = 1))]
    pub from: String,

    /// Edge label (always "RESULTED_IN")
    #[serde(rename = "_label")]
    #[validate(length(min = 1))]
    pub label: String,

    /// Player's placement in the contest
    #[validate(range(min = 1))]
    pub place: i32,

    /// Result description (e.g., "won", "lost")
    #[validate(length(min = 1))]
    pub result: String,
}

impl PlayedAt {
    /// Creates a new played at relation with validation
    pub fn new(id: String, rev: String, to: String, from: String) -> Result<Self> {
        let relation = Self {
            id,
            rev,
            to,
            from,
            label: "PLAYED_AT".to_string(),
        };
        relation.validate_fields()?;
        Ok(relation)
    }

    /// Validates the played at relation data
    pub fn validate_fields(&self) -> Result<()> {
        self.validate()
            .map_err(|e| SharedError::Validation(e.to_string()))
    }
}

impl PlayedWith {
    /// Creates a new played with relation with validation
    /// This represents a relation between a contest and a game
    pub fn new(id: String, rev: String, to: String, from: String) -> Result<Self> {
        let relation = Self {
            id,
            rev,
            to,
            from,
            label: "PLAYED_WITH".to_string(),
        };
        relation.validate_fields()?;
        Ok(relation)
    }

    /// Validates the played with relation data
    pub fn validate_fields(&self) -> Result<()> {
        self.validate()
            .map_err(|e| SharedError::Validation(e.to_string()))
    }
}

impl ResultedIn {
    /// Creates a new resulted in relation with validation
    /// This represents a relation from a contest to a player with their result
    pub fn new(
        id: String,
        rev: String,
        to: String,
        from: String,
        place: i32,
        result: String,
    ) -> Result<Self> {
        let relation = Self {
            id,
            rev,
            to,
            from,
            label: "RESULTED_IN".to_string(),
            place,
            result,
        };
        relation.validate_fields()?;
        Ok(relation)
    }

    /// Validates the resulted in relation data
    pub fn validate_fields(&self) -> Result<()> {
        self.validate()
            .map_err(|e| SharedError::Validation(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use test_log::test;
    use validator::Validate;

    fn create_test_played_at() -> PlayedAt {
        PlayedAt {
            id: "played_at/test".to_string(),
            rev: "1".to_string(),
            to: "venue/test-venue".to_string(),
            from: "contest/test-contest".to_string(),
            label: "PLAYED_AT".to_string(),
        }
    }

    fn create_test_played_with() -> PlayedWith {
        PlayedWith {
            id: "played_with/test".to_string(),
            rev: "1".to_string(),
            to: "game/test-game".to_string(),
            from: "contest/test-contest".to_string(),
            label: "PLAYED_WITH".to_string(),
        }
    }

    fn create_test_resulted_in() -> ResultedIn {
        ResultedIn {
            id: "resulted_in/test".to_string(),
            rev: "1".to_string(),
            to: "player/test-player".to_string(),
            from: "contest/test-contest".to_string(),
            label: "RESULTED_IN".to_string(),
            place: 1,
            result: "won".to_string(),
        }
    }

    // PlayedAt Tests
    #[test]
    fn test_played_at_creation() {
        let relation = create_test_played_at();
        assert_eq!(relation.id, "played_at/test");
        assert_eq!(relation.to, "venue/test-venue");
        assert_eq!(relation.from, "contest/test-contest");
        assert_eq!(relation.label, "PLAYED_AT");
    }

    #[test]
    fn test_played_at_validation_success() {
        let relation = create_test_played_at();
        assert!(relation.validate().is_ok());
    }

    #[test]
    fn test_played_at_validation_empty_to() {
        let mut relation = create_test_played_at();
        relation.to = "".to_string();
        let result = relation.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("_to"));
    }

    #[test]
    fn test_played_at_validation_empty_from() {
        let mut relation = create_test_played_at();
        relation.from = "".to_string();
        let result = relation.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("_from"));
    }

    #[test]
    fn test_played_at_validation_empty_label() {
        let mut relation = create_test_played_at();
        relation.label = "".to_string();
        let result = relation.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("_label"));
    }

    #[test]
    fn test_played_at_serialization() {
        let relation = create_test_played_at();
        let json = serde_json::to_string(&relation).unwrap();
        let deserialized: PlayedAt = serde_json::from_str(&json).unwrap();
        assert_eq!(relation.id, deserialized.id);
        assert_eq!(relation.to, deserialized.to);
        assert_eq!(relation.from, deserialized.from);
        assert_eq!(relation.label, deserialized.label);
    }

    #[test]
    fn test_played_at_new_with_validation() {
        let result = PlayedAt::new(
            "played_at/test".to_string(),
            "1".to_string(),
            "venue/test-venue".to_string(),
            "contest/test-contest".to_string(),
        );
        assert!(result.is_ok());
        let relation = result.unwrap();
        assert_eq!(relation.label, "PLAYED_AT");
    }

    #[test]
    fn test_played_at_new_with_invalid_data() {
        let result = PlayedAt::new(
            "played_at/test".to_string(),
            "1".to_string(),
            "".to_string(), // Invalid: empty to
            "contest/test-contest".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_played_at_validate_fields() {
        let relation = create_test_played_at();
        assert!(relation.validate_fields().is_ok());
    }

    // PlayedWith Tests
    #[test]
    fn test_played_with_creation() {
        let relation = create_test_played_with();
        assert_eq!(relation.id, "played_with/test");
        assert_eq!(relation.to, "game/test-game");
        assert_eq!(relation.from, "contest/test-contest");
        assert_eq!(relation.label, "PLAYED_WITH");
    }

    #[test]
    fn test_played_with_validation_success() {
        let relation = create_test_played_with();
        assert!(relation.validate().is_ok());
    }

    #[test]
    fn test_played_with_validation_empty_to() {
        let mut relation = create_test_played_with();
        relation.to = "".to_string();
        let result = relation.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("_to"));
    }

    #[test]
    fn test_played_with_serialization() {
        let relation = create_test_played_with();
        let json = serde_json::to_string(&relation).unwrap();
        let deserialized: PlayedWith = serde_json::from_str(&json).unwrap();
        assert_eq!(relation.id, deserialized.id);
        assert_eq!(relation.to, deserialized.to);
        assert_eq!(relation.from, deserialized.from);
        assert_eq!(relation.label, deserialized.label);
    }

    #[test]
    fn test_played_with_new_with_validation() {
        let result = PlayedWith::new(
            "played_with/test".to_string(),
            "1".to_string(),
            "game/test-game".to_string(),
            "contest/test-contest".to_string(),
        );
        assert!(result.is_ok());
        let relation = result.unwrap();
        assert_eq!(relation.label, "PLAYED_WITH");
    }

    #[test]
    fn test_played_with_validate_fields() {
        let relation = create_test_played_with();
        assert!(relation.validate_fields().is_ok());
    }

    // ResultedIn Tests
    #[test]
    fn test_resulted_in_creation() {
        let relation = create_test_resulted_in();
        assert_eq!(relation.id, "resulted_in/test");
        assert_eq!(relation.to, "player/test-player");
        assert_eq!(relation.from, "contest/test-contest");
        assert_eq!(relation.label, "RESULTED_IN");
        assert_eq!(relation.place, 1);
        assert_eq!(relation.result, "won");
    }

    #[test]
    fn test_resulted_in_validation_success() {
        let relation = create_test_resulted_in();
        assert!(relation.validate().is_ok());
    }

    #[test]
    fn test_resulted_in_validation_invalid_place() {
        let mut relation = create_test_resulted_in();
        relation.place = 0; // Invalid: place must be >= 1
        let result = relation.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("place"));
    }

    #[test]
    fn test_resulted_in_validation_empty_result() {
        let mut relation = create_test_resulted_in();
        relation.result = "".to_string();
        let result = relation.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("result"));
    }

    #[test]
    fn test_resulted_in_validation_empty_to() {
        let mut relation = create_test_resulted_in();
        relation.to = "".to_string();
        let result = relation.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("_to"));
    }

    #[test]
    fn test_resulted_in_serialization() {
        let relation = create_test_resulted_in();
        let json = serde_json::to_string(&relation).unwrap();
        let deserialized: ResultedIn = serde_json::from_str(&json).unwrap();
        assert_eq!(relation.id, deserialized.id);
        assert_eq!(relation.to, deserialized.to);
        assert_eq!(relation.from, deserialized.from);
        assert_eq!(relation.label, deserialized.label);
        assert_eq!(relation.place, deserialized.place);
        assert_eq!(relation.result, deserialized.result);
    }

    #[test]
    fn test_resulted_in_new_with_validation() {
        let result = ResultedIn::new(
            "resulted_in/test".to_string(),
            "1".to_string(),
            "player/test-player".to_string(),
            "contest/test-contest".to_string(),
            1,
            "won".to_string(),
        );
        assert!(result.is_ok());
        let relation = result.unwrap();
        assert_eq!(relation.label, "RESULTED_IN");
        assert_eq!(relation.place, 1);
        assert_eq!(relation.result, "won");
    }

    #[test]
    fn test_resulted_in_new_with_invalid_place() {
        let result = ResultedIn::new(
            "resulted_in/test".to_string(),
            "1".to_string(),
            "player/test-player".to_string(),
            "contest/test-contest".to_string(),
            0, // Invalid: place must be >= 1
            "won".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_resulted_in_new_with_empty_result() {
        let result = ResultedIn::new(
            "resulted_in/test".to_string(),
            "1".to_string(),
            "player/test-player".to_string(),
            "contest/test-contest".to_string(),
            1,
            "".to_string(), // Invalid: empty result
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_resulted_in_validate_fields() {
        let relation = create_test_resulted_in();
        assert!(relation.validate_fields().is_ok());
    }

    // Edge case tests
    #[test]
    fn test_relations_with_special_characters() {
        // Test with special characters in IDs
        let played_at = PlayedAt {
            id: "played_at/test-relation_123".to_string(),
            rev: "1".to_string(),
            to: "venue/test-venue_456".to_string(),
            from: "contest/test-contest_789".to_string(),
            label: "PLAYED_AT".to_string(),
        };
        assert!(played_at.validate().is_ok());

        let resulted_in = ResultedIn {
            id: "resulted_in/test-relation".to_string(),
            rev: "1".to_string(),
            to: "player/test-player_123".to_string(),
            from: "contest/test-contest_456".to_string(),
            label: "RESULTED_IN".to_string(),
            place: 1,
            result: "won (tie-breaker)".to_string(),
        };
        assert!(resulted_in.validate().is_ok());
    }

    #[test]
    fn test_relations_with_different_place_values() {
        let relation = ResultedIn {
            id: "resulted_in/test".to_string(),
            rev: "1".to_string(),
            to: "player/test-player".to_string(),
            from: "contest/test-contest".to_string(),
            label: "RESULTED_IN".to_string(),
            place: 999, // Large but valid place
            result: "participated".to_string(),
        };
        assert!(relation.validate().is_ok());
    }

    #[test]
    fn test_relations_id_formats() {
        let played_at = create_test_played_at();
        assert!(played_at.id.starts_with("played_at/"));

        let played_with = create_test_played_with();
        assert!(played_with.id.starts_with("played_with/"));

        let resulted_in = create_test_resulted_in();
        assert!(resulted_in.id.starts_with("resulted_in/"));
    }

    #[test]
    fn test_relations_rev_formats() {
        let played_at = create_test_played_at();
        assert!(played_at.rev.parse::<i32>().is_ok());

        let played_with = create_test_played_with();
        assert!(played_with.rev.parse::<i32>().is_ok());

        let resulted_in = create_test_resulted_in();
        assert!(resulted_in.rev.parse::<i32>().is_ok());
    }
}
