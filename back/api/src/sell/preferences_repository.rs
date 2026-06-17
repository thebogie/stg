//! Per-player BGG sell defaults persistence.

use crate::db::Db;
use crate::surreal_helpers::{record_id_from_field, record_id_from_row, record_id_to_key};
use chrono::Utc;
use shared::dto::sell_preferences::SellPreferencesDto;
use shared::models::sell_preferences::SellPreferences;

#[derive(Clone)]
pub struct SellPreferencesRepositoryImpl {
    pub db: Db,
    pub ns: Option<String>,
    pub db_name: Option<String>,
}

impl SellPreferencesRepositoryImpl {
    pub fn new_with_scope(db: Db, ns: String, db_name: String) -> Self {
        Self {
            db,
            ns: Some(ns),
            db_name: Some(db_name),
        }
    }

    fn query_with_scope(&self, core: &str) -> String {
        if let (Some(ref ns), Some(ref db_name)) = (&self.ns, &self.db_name) {
            let ns_ok = ns.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            let db_ok = db_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_');
            if ns_ok && db_ok {
                return format!("USE NS {}; USE DB {}; {}", ns, db_name, core);
            }
        }
        core.to_string()
    }

    fn scope_result_index(&self) -> usize {
        if self.ns.is_some() && self.db_name.is_some() {
            2
        } else {
            0
        }
    }

    fn player_key(player_id: &str) -> String {
        record_id_to_key(player_id, "player")
    }

    fn value_to_prefs(v: &serde_json::Value) -> Option<SellPreferences> {
        let id = record_id_from_row(v, None).unwrap_or_default();
        let parse_dt = |k: &str| {
            v.get(k)
                .and_then(|x| x.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
        };
        Some(SellPreferences {
            id,
            player_id: record_id_from_field(v, "player_id")
                .or_else(|| {
                    v.get("player_id")
                        .and_then(|x| x.as_str())
                        .map(|s| s.replace(':', "/"))
                })
                .unwrap_or_default(),
            currency: v
                .get("currency")
                .and_then(|x| x.as_str())
                .unwrap_or("USD")
                .to_string(),
            condition: v
                .get("condition")
                .and_then(|x| x.as_str())
                .unwrap_or(shared::models::sell_preferences::bgg_condition::VERY_GOOD)
                .to_string(),
            payment_paypal: v
                .get("payment_paypal")
                .and_then(|x| x.as_bool())
                .unwrap_or(true),
            payment_other: v
                .get("payment_other")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            item_location: v
                .get("item_location")
                .and_then(|x| x.as_str())
                .unwrap_or("United States")
                .to_string(),
            ship_to: v
                .get("ship_to")
                .and_then(|x| x.as_str())
                .unwrap_or("United States only")
                .to_string(),
            seller_notes: v
                .get("seller_notes")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            bgg_username: v
                .get("bgg_username")
                .and_then(|x| x.as_str())
                .map(String::from),
            updated_at: parse_dt("updated_at").unwrap_or_else(Utc::now),
        })
    }

    pub async fn get_or_default(&self, player_id: &str) -> SellPreferencesDto {
        let player_key = Self::player_key(player_id);
        if player_key.is_empty() {
            return SellPreferencesDto::from(&SellPreferences::default());
        }
        let sql = self.query_with_scope(
            "SELECT * FROM sell_preferences WHERE player_id = type::record('player', $player_key) LIMIT 1",
        );
        let idx = self.scope_result_index();
        let mut res = match self
            .db
            .query(&sql)
            .bind(("player_key", player_key))
            .await
        {
            Ok(r) => r,
            Err(_) => return SellPreferencesDto::from(&SellPreferences::default()),
        };
        let rows: Vec<serde_json::Value> = res.take(idx).unwrap_or_default();
        rows.first()
            .and_then(Self::value_to_prefs)
            .map(|p| SellPreferencesDto::from(&p))
            .unwrap_or_else(|| SellPreferencesDto::from(&SellPreferences::default()))
    }

    pub async fn has_preferences(&self, player_id: &str) -> bool {
        let player_key = Self::player_key(player_id);
        if player_key.is_empty() {
            return false;
        }
        let sql = self.query_with_scope(
            "SELECT count() AS c FROM sell_preferences WHERE player_id = type::record('player', $player_key) GROUP ALL",
        );
        let idx = self.scope_result_index();
        let mut res = match self
            .db
            .query(&sql)
            .bind(("player_key", player_key))
            .await
        {
            Ok(r) => r,
            Err(_) => return false,
        };
        let rows: Vec<serde_json::Value> = res.take(idx).unwrap_or_default();
        rows.first()
            .and_then(|r| r.get("c"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0)
            > 0
    }

    pub async fn upsert(
        &self,
        player_id: &str,
        dto: SellPreferencesDto,
    ) -> Result<SellPreferencesDto, String> {
        let player_key = Self::player_key(player_id);
        if player_key.is_empty() {
            return Err("invalid player".to_string());
        }
        let now = Utc::now();
        let fields = serde_json::json!({
            "player_id": format!("player/{player_key}"),
            "currency": dto.currency,
            "condition": dto.condition,
            "payment_paypal": dto.payment_paypal,
            "payment_other": dto.payment_other,
            "item_location": dto.item_location,
            "ship_to": dto.ship_to,
            "seller_notes": dto.seller_notes,
            "bgg_username": dto.bgg_username,
            "updated_at": now.to_rfc3339(),
        });

        let existing = self.has_preferences(player_id).await;
        if existing {
            let sql = self.query_with_scope(
                "UPDATE sell_preferences SET currency = $currency, condition = $condition, \
                 payment_paypal = $payment_paypal, payment_other = $payment_other, \
                 item_location = $item_location, ship_to = $ship_to, seller_notes = $seller_notes, \
                 bgg_username = $bgg_username, updated_at = type::datetime($updated_at) \
                 WHERE player_id = type::record('player', $player_key)",
            );
            let idx = self.scope_result_index();
            let _ = self
                .db
                .query(&sql)
                .bind(("player_key", player_key.clone()))
                .bind(("currency", fields["currency"].clone()))
                .bind(("condition", fields["condition"].clone()))
                .bind(("payment_paypal", fields["payment_paypal"].clone()))
                .bind(("payment_other", fields["payment_other"].clone()))
                .bind(("item_location", fields["item_location"].clone()))
                .bind(("ship_to", fields["ship_to"].clone()))
                .bind(("seller_notes", fields["seller_notes"].clone()))
                .bind(("bgg_username", fields["bgg_username"].clone()))
                .bind(("updated_at", now.to_rfc3339()))
                .await
                .map_err(|e| e.to_string())?
                .take::<Vec<serde_json::Value>>(idx);
        } else {
            let sql = self.query_with_scope(
                "CREATE sell_preferences CONTENT {\
                 player_id: type::record('player', $player_key),\
                 currency: $currency,\
                 condition: $condition,\
                 payment_paypal: $payment_paypal,\
                 payment_other: $payment_other,\
                 item_location: $item_location,\
                 ship_to: $ship_to,\
                 seller_notes: $seller_notes,\
                 bgg_username: $bgg_username,\
                 updated_at: type::datetime($updated_at)\
                 }",
            );
            let idx = self.scope_result_index();
            let _ = self
                .db
                .query(&sql)
                .bind(("player_key", player_key))
                .bind(("currency", fields["currency"].clone()))
                .bind(("condition", fields["condition"].clone()))
                .bind(("payment_paypal", fields["payment_paypal"].clone()))
                .bind(("payment_other", fields["payment_other"].clone()))
                .bind(("item_location", fields["item_location"].clone()))
                .bind(("ship_to", fields["ship_to"].clone()))
                .bind(("seller_notes", fields["seller_notes"].clone()))
                .bind(("bgg_username", fields["bgg_username"].clone()))
                .bind(("updated_at", now.to_rfc3339()))
                .await
                .map_err(|e| e.to_string())?
                .take::<Vec<serde_json::Value>>(idx);
        }
        Ok(self.get_or_default(player_id).await)
    }
}
