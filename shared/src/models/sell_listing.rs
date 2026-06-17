//! Sell-a-Game listing workflow (ephemeral photos → AI draft → BGG marketplace).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Stored in SurrealDB `sell_listing.status`.
pub mod listing_status {
    pub const DRAFT_CREATED: &str = "draft_created";
    pub const PHOTOS_UPLOADED: &str = "photos_uploaded";
    pub const EXTRACTION_READY: &str = "extraction_ready";
    pub const AI_DRAFT: &str = "ai_draft";
    pub const HUMAN_REVIEWED: &str = "human_reviewed";
    pub const BGG_MATCHED: &str = "bgg_matched";
    pub const AUTOMATION_READY: &str = "automation_ready";
    pub const BGG_PREVIEW: &str = "bgg_preview";
    pub const SUBMITTED: &str = "submitted";
    pub const CANCELLED: &str = "cancelled";
}

/// Named human approval gates.
pub mod checkpoint {
    pub const PREFERENCES: &str = "preferences";
    pub const PHOTOS: &str = "photos";
    pub const LISTING: &str = "listing";
    pub const AUTOMATION: &str = "automation";
    // Legacy (AI flow)
    pub const AI_REVIEW: &str = "ai_review";
    pub const BGG_MATCH: &str = "bgg_match";
    pub const MARKETPLACE: &str = "marketplace";
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckpointApproval {
    pub checkpoint: String,
    pub approved_at: DateTime<Utc>,
    pub approved_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SellListingPhoto {
    #[serde(rename = "_id", default)]
    pub id: String,
    pub listing_id: String,
    pub sort_order: u32,
    pub content_type: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SellListing {
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(rename = "_rev", default)]
    pub rev: String,
    pub seller_id: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub condition_notes: Option<String>,
    /// BGG condition enum: new, like_new, very_good, good, acceptable
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub price_cents: Option<i64>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub shipping_notes: Option<String>,
    #[serde(default)]
    pub bgg_id: Option<i32>,
    #[serde(default)]
    pub game_name: Option<String>,
    #[serde(default)]
    pub edition_notes: Option<String>,
    #[serde(default)]
    pub missing_components: Vec<String>,
    #[serde(default)]
    pub ai_confidence: Option<f64>,
    #[serde(default)]
    pub ai_questions: Vec<String>,
    #[serde(default)]
    pub ai_warnings: Vec<String>,
    #[serde(default)]
    pub bgg_listing_url: Option<String>,
    #[serde(default)]
    pub checkpoint_approvals: Vec<CheckpointApproval>,
    #[serde(default)]
    pub photo_count: u32,
}

impl SellListing {
    pub fn has_checkpoint(&self, name: &str) -> bool {
        self.checkpoint_approvals
            .iter()
            .any(|a| a.checkpoint == name)
    }
}
