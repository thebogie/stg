//! Per-player defaults for BGG GeekMarket listings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// BGG marketplace condition values (GeekMarket form).
pub mod bgg_condition {
    pub const NEW: &str = "new";
    pub const LIKE_NEW: &str = "like_new";
    pub const VERY_GOOD: &str = "very_good";
    pub const GOOD: &str = "good";
    pub const ACCEPTABLE: &str = "acceptable";

    pub fn label(value: &str) -> &'static str {
        match value {
            NEW => "New",
            LIKE_NEW => "Like New",
            VERY_GOOD => "Very Good",
            GOOD => "Good",
            ACCEPTABLE => "Acceptable",
            _ => "Very Good",
        }
    }

    pub fn all() -> &'static [&'static str] {
        &[NEW, LIKE_NEW, VERY_GOOD, GOOD, ACCEPTABLE]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SellPreferences {
    #[serde(rename = "_id", default)]
    pub id: String,
    pub player_id: String,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default = "default_condition")]
    pub condition: String,
    #[serde(default = "default_true")]
    pub payment_paypal: bool,
    #[serde(default)]
    pub payment_other: bool,
    #[serde(default)]
    pub item_location: String,
    #[serde(default)]
    pub ship_to: String,
    #[serde(default)]
    pub seller_notes: String,
    /// Remembered BGG login name only — password is never stored.
    #[serde(default)]
    pub bgg_username: Option<String>,
    pub updated_at: DateTime<Utc>,
}

fn default_currency() -> String {
    "USD".to_string()
}

fn default_condition() -> String {
    bgg_condition::VERY_GOOD.to_string()
}

fn default_true() -> bool {
    true
}

impl Default for SellPreferences {
    fn default() -> Self {
        Self {
            id: String::new(),
            player_id: String::new(),
            currency: default_currency(),
            condition: default_condition(),
            payment_paypal: true,
            payment_other: false,
            item_location: "United States".to_string(),
            ship_to: "United States only".to_string(),
            seller_notes: String::new(),
            bgg_username: None,
            updated_at: Utc::now(),
        }
    }
}
