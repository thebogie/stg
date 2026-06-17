//! API DTOs for Sell a Game workflow.

use crate::models::sell_listing::{
    checkpoint, listing_status, CheckpointApproval, SellListing, SellListingPhoto,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate)]
pub struct SellListingDto {
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(default)]
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
    #[serde(default)]
    pub photos: Vec<SellListingPhotoDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SellListingPhotoDto {
    #[serde(rename = "_id", default)]
    pub id: String,
    pub listing_id: String,
    pub sort_order: u32,
    pub content_type: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateSellListingDraftRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub condition_notes: Option<String>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub price_cents: Option<i64>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub shipping_notes: Option<String>,
    #[serde(default)]
    pub game_name: Option<String>,
    #[serde(default)]
    pub edition_notes: Option<String>,
    #[serde(default)]
    pub missing_components: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BggMatchRequest {
    pub bgg_id: i32,
    #[validate(length(min = 1, max = 256))]
    pub game_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiExtractionResultDto {
    pub listing: SellListingDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clarify: Option<AiClarifyDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiClarifyDto {
    pub question: String,
    pub choices: Vec<AiClarifyChoiceDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiClarifyChoiceDto {
    pub label: String,
    pub question: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationResultRequest {
    pub success: bool,
    #[serde(default)]
    pub bgg_listing_url: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
    /// Operator confirmed final BGG submit.
    #[serde(default)]
    pub submitted_on_bgg: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BggExportPayload {
    pub listing_id: String,
    pub title: String,
    pub description: String,
    pub condition: String,
    pub condition_notes: String,
    pub price_cents: i64,
    pub currency: String,
    pub shipping_notes: String,
    pub bgg_id: i32,
    pub game_name: String,
    pub edition_notes: String,
    pub missing_components: Vec<String>,
    pub payment_paypal: bool,
    pub payment_other: bool,
    pub item_location: String,
    pub ship_to: String,
    pub seller_notes: String,
    pub photo_paths: Vec<String>,
}

impl From<&SellListing> for SellListingDto {
    fn from(m: &SellListing) -> Self {
        SellListingDto {
            id: m.id.clone(),
            seller_id: m.seller_id.clone(),
            status: m.status.clone(),
            created_at: m.created_at,
            updated_at: m.updated_at,
            expires_at: m.expires_at,
            title: m.title.clone(),
            description: m.description.clone(),
            condition_notes: m.condition_notes.clone(),
            condition: m.condition.clone(),
            price_cents: m.price_cents,
            currency: m.currency.clone(),
            shipping_notes: m.shipping_notes.clone(),
            bgg_id: m.bgg_id,
            game_name: m.game_name.clone(),
            edition_notes: m.edition_notes.clone(),
            missing_components: m.missing_components.clone(),
            ai_confidence: m.ai_confidence,
            ai_questions: m.ai_questions.clone(),
            ai_warnings: m.ai_warnings.clone(),
            bgg_listing_url: m.bgg_listing_url.clone(),
            checkpoint_approvals: m.checkpoint_approvals.clone(),
            photo_count: m.photo_count,
            photos: Vec::new(),
        }
    }
}

impl From<&SellListingPhoto> for SellListingPhotoDto {
    fn from(p: &SellListingPhoto) -> Self {
        SellListingPhotoDto {
            id: p.id.clone(),
            listing_id: p.listing_id.clone(),
            sort_order: p.sort_order,
            content_type: p.content_type.clone(),
            size_bytes: p.size_bytes,
            created_at: p.created_at,
            preview_url: None,
        }
    }
}

/// Valid next statuses after a checkpoint approval.
pub fn status_after_checkpoint(cp: &str) -> Option<&'static str> {
    match cp {
        checkpoint::PREFERENCES => Some(listing_status::DRAFT_CREATED),
        checkpoint::PHOTOS => Some(listing_status::PHOTOS_UPLOADED),
        checkpoint::LISTING => Some(listing_status::AUTOMATION_READY),
        checkpoint::AUTOMATION => Some(listing_status::BGG_PREVIEW),
        checkpoint::AI_REVIEW => Some(listing_status::HUMAN_REVIEWED),
        checkpoint::BGG_MATCH => Some(listing_status::BGG_MATCHED),
        checkpoint::MARKETPLACE => Some(listing_status::AUTOMATION_READY),
        _ => None,
    }
}
