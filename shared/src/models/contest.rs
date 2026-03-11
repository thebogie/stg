use crate::{Result, SharedError};
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Represents a contest in the system
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct Contest {
    /// ArangoDB document ID (format: "contest/{timestamp}")
    #[serde(rename = "_id")]
    pub id: String,

    /// ArangoDB document revision
    #[serde(rename = "_rev")]
    pub rev: String,

    /// Name of the contest
    #[validate(length(
        min = 1,
        max = 1000,
        message = "Name must be between 1 and 1000 characters"
    ))]
    pub name: String,

    /// Contest start time (UTC)
    pub start: DateTime<FixedOffset>,

    /// Contest end time (UTC)
    pub stop: DateTime<FixedOffset>,

    /// ID of the player who created this contest
    pub creator_id: String,

    /// When this contest was created (UTC)
    pub created_at: DateTime<FixedOffset>,
}

impl Contest {
    /// Creates a new contest with validation
    pub fn new(
        id: String,
        rev: String,
        start: DateTime<FixedOffset>,
        stop: DateTime<FixedOffset>,
        name: String,
        creator_id: String,
        created_at: DateTime<FixedOffset>,
    ) -> Result<Self> {
        let contest = Self {
            id,
            rev,
            start,
            stop,
            name,
            creator_id,
            created_at,
        };
        contest.validate_fields()?;
        Ok(contest)
    }

    /// Validates the contest data
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

    fn create_test_contest() -> Contest {
        Contest {
            id: "contest/test-contest".to_string(),
            rev: "1".to_string(),
            name: "Test Contest".to_string(),
            start: DateTime::parse_from_rfc3339("2023-07-15T14:00:00Z").unwrap(),
            stop: DateTime::parse_from_rfc3339("2023-07-15T16:00:00Z").unwrap(),
            creator_id: "player/test-creator".to_string(),
            created_at: DateTime::parse_from_rfc3339("2023-07-15T10:00:00Z").unwrap(),
        }
    }

    #[test]
    fn test_contest_creation() {
        let contest = create_test_contest();
        assert_eq!(contest.name, "Test Contest");
        assert_eq!(contest.id, "contest/test-contest");
        assert_eq!(contest.rev, "1");
    }

    #[test]
    fn test_contest_validation_success() {
        let contest = create_test_contest();
        assert!(contest.validate().is_ok());
    }

    #[test]
    fn test_contest_validation_empty_name() {
        let mut contest = create_test_contest();
        contest.name = "".to_string();
        let result = contest.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("name"));
    }

    #[test]
    fn test_contest_validation_very_long_name() {
        let mut contest = create_test_contest();
        contest.name = "A".repeat(1001); // Very long name
        let result = contest.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("name"));
    }

    #[test]
    fn test_contest_serialization() {
        let contest = create_test_contest();
        let json = serde_json::to_string(&contest).unwrap();
        let deserialized: Contest = serde_json::from_str(&json).unwrap();
        assert_eq!(contest.id, deserialized.id);
        assert_eq!(contest.name, deserialized.name);
        assert_eq!(contest.start, deserialized.start);
        assert_eq!(contest.stop, deserialized.stop);
    }

    #[test]
    fn test_contest_id_format() {
        let contest = create_test_contest();
        assert!(contest.id.starts_with("contest/"));
    }

    #[test]
    fn test_contest_rev_format() {
        let contest = create_test_contest();
        assert!(contest.rev.parse::<i32>().is_ok());
    }

    #[test]
    fn test_contest_with_different_timezones() {
        let contest = Contest {
            id: "contest/test".to_string(),
            rev: "1".to_string(),
            name: "Timezone Test Contest".to_string(),
            start: DateTime::parse_from_rfc3339("2023-07-15T14:00:00-05:00").unwrap(),
            stop: DateTime::parse_from_rfc3339("2023-07-15T16:00:00-05:00").unwrap(),
            creator_id: "player/test-creator".to_string(),
            created_at: DateTime::parse_from_rfc3339("2023-07-15T10:00:00Z").unwrap(),
        };
        assert!(contest.validate().is_ok());
    }

    #[test]
    fn test_contest_with_special_characters_in_name() {
        let mut contest = create_test_contest();
        contest.name = "Test Contest & Tournament (2023)".to_string();
        assert!(contest.validate().is_ok());
    }

    #[test]
    fn test_contest_new_with_validation() {
        let start = DateTime::parse_from_rfc3339("2023-07-15T14:00:00Z").unwrap();
        let stop = DateTime::parse_from_rfc3339("2023-07-15T16:00:00Z").unwrap();

        let result = Contest::new(
            "contest/test".to_string(),
            "1".to_string(),
            start,
            stop,
            "Valid Contest".to_string(),
            "player/test-creator".to_string(),
            DateTime::parse_from_rfc3339("2023-07-15T10:00:00Z").unwrap(),
        );

        assert!(result.is_ok());
        let contest = result.unwrap();
        assert_eq!(contest.name, "Valid Contest");
    }

    #[test]
    fn test_contest_new_with_invalid_name() {
        let start = DateTime::parse_from_rfc3339("2023-07-15T14:00:00Z").unwrap();
        let stop = DateTime::parse_from_rfc3339("2023-07-15T16:00:00Z").unwrap();

        let result = Contest::new(
            "contest/test".to_string(),
            "1".to_string(),
            start,
            stop,
            "".to_string(), // Invalid: empty name
            "player/test-creator".to_string(),
            DateTime::parse_from_rfc3339("2023-07-15T10:00:00Z").unwrap(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_contest_validate_fields() {
        let contest = create_test_contest();
        assert!(contest.validate_fields().is_ok());
    }

    #[test]
    fn test_contest_validate_fields_invalid() {
        let mut contest = create_test_contest();
        contest.name = "".to_string();
        assert!(contest.validate_fields().is_err());
    }

    #[test]
    fn test_contest_creator_tracking() {
        let contest = create_test_contest();
        assert_eq!(contest.creator_id, "player/test-creator");
        assert_eq!(
            contest.created_at,
            DateTime::parse_from_rfc3339("2023-07-15T10:00:00Z").unwrap()
        );
    }

    #[test]
    fn test_contest_new_with_creator_fields() {
        let start = DateTime::parse_from_rfc3339("2023-07-15T14:00:00Z").unwrap();
        let stop = DateTime::parse_from_rfc3339("2023-07-15T16:00:00Z").unwrap();
        let created_at = DateTime::parse_from_rfc3339("2023-07-15T10:00:00Z").unwrap();

        let result = Contest::new(
            "contest/test".to_string(),
            "1".to_string(),
            start,
            stop,
            "Creator Test Contest".to_string(),
            "player/test-creator".to_string(),
            created_at,
        );

        assert!(result.is_ok());
        let contest = result.unwrap();
        assert_eq!(contest.name, "Creator Test Contest");
        assert_eq!(contest.creator_id, "player/test-creator");
        assert_eq!(contest.created_at, created_at);
    }

    #[test]
    fn test_contest_serialization_with_creator() {
        let contest = create_test_contest();
        let json = serde_json::to_string(&contest).unwrap();
        let deserialized: Contest = serde_json::from_str(&json).unwrap();

        assert_eq!(contest.id, deserialized.id);
        assert_eq!(contest.name, deserialized.name);
        assert_eq!(contest.creator_id, deserialized.creator_id);
        assert_eq!(contest.created_at, deserialized.created_at);
    }
}
