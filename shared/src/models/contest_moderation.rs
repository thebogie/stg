//! Contest publication / moderation (approve before public listing).

/// Stored in SurrealDB `contest.moderation_status`.
pub mod moderation_status {
    pub const PENDING: &str = "pending";
    pub const APPROVED: &str = "approved";
    pub const REJECTED: &str = "rejected";
}
