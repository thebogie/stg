//! API DTOs for per-player BGG sell defaults.

use crate::models::sell_preferences::SellPreferences;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate)]
pub struct SellPreferencesDto {
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub condition: String,
    #[serde(default)]
    pub payment_paypal: bool,
    #[serde(default)]
    pub payment_other: bool,
    #[serde(default)]
    pub item_location: String,
    #[serde(default)]
    pub ship_to: String,
    #[serde(default)]
    pub seller_notes: String,
    #[serde(default)]
    pub bgg_username: Option<String>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BggAutomateRequest {
    #[validate(length(min = 1, max = 128))]
    pub bgg_username: String,
    #[validate(length(min = 1, max = 256))]
    pub bgg_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BggAutomateResponse {
    pub listing_id: String,
    pub status: String,
    pub message: String,
    /// Set when `PLAYWRIGHT_MODE=queue`; poll `PlaywrightJobStatusDto` until terminal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaywrightJobStatusDto {
    pub job_id: String,
    pub listing_id: String,
    pub job_type: String,
    /// `queued` | `running` | `completed` | `failed`
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub mod playwright_job_status {
    pub const QUEUED: &str = "queued";
    pub const RUNNING: &str = "running";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
}

pub mod playwright_job_type {
    pub const BGG_GEEKMARKET_FILL: &str = "bgg.geekmarket.fill";
    pub const SMOKE_STG: &str = "smoke.stg";
}

impl Default for SellPreferencesDto {
    fn default() -> Self {
        let p = SellPreferences::default();
        SellPreferencesDto::from(&p)
    }
}

impl From<&SellPreferences> for SellPreferencesDto {
    fn from(p: &SellPreferences) -> Self {
        SellPreferencesDto {
            currency: p.currency.clone(),
            condition: p.condition.clone(),
            payment_paypal: p.payment_paypal,
            payment_other: p.payment_other,
            item_location: p.item_location.clone(),
            ship_to: p.ship_to.clone(),
            seller_notes: p.seller_notes.clone(),
            bgg_username: p.bgg_username.clone(),
            updated_at: Some(p.updated_at),
        }
    }
}

impl From<SellPreferencesDto> for SellPreferences {
    fn from(d: SellPreferencesDto) -> Self {
        SellPreferences {
            id: String::new(),
            player_id: String::new(),
            currency: d.currency,
            condition: d.condition,
            payment_paypal: d.payment_paypal,
            payment_other: d.payment_other,
            item_location: d.item_location,
            ship_to: d.ship_to,
            seller_notes: d.seller_notes,
            bgg_username: d.bgg_username,
            updated_at: d.updated_at.unwrap_or_else(Utc::now),
        }
    }
}
