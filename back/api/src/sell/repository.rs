//! SurrealDB persistence for sell listings.

use crate::db::Db;
use crate::sell::ai_extraction;
use crate::sell::image::{
    delete_all_listing_photos, delete_photo_file, max_photo_count, read_photo_variant,
    ttl_hours, write_photo_atomic, SellPhotoVariant,
};
use crate::surreal_helpers::{record_id_from_field, record_id_from_row, record_id_to_key};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use shared::dto::sell_listing::{
    status_after_checkpoint, BggExportPayload, SellListingDto, SellListingPhotoDto,
    UpdateSellListingDraftRequest,
};
use shared::models::sell_listing::{
    checkpoint, listing_status, CheckpointApproval, SellListing, SellListingPhoto,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct SellListingRepositoryImpl {
    pub db: Db,
    pub ns: Option<String>,
    pub db_name: Option<String>,
}

impl SellListingRepositoryImpl {
    pub fn new(db: Db) -> Self {
        Self {
            db,
            ns: None,
            db_name: None,
        }
    }

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

    fn listing_key(id: &str) -> String {
        record_id_to_key(id, "sell_listing")
    }

    fn photo_key(id: &str) -> String {
        record_id_to_key(id, "sell_listing_photo")
    }

    fn value_to_listing(v: &serde_json::Value) -> Option<SellListing> {
        let id = record_id_from_row(v, None)?;
        let parse_dt = |k: &str| {
            v.get(k)
                .and_then(|x| x.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
        };
        let approvals: Vec<CheckpointApproval> = v
            .get("checkpoint_approvals")
            .and_then(|x| serde_json::from_value(x.clone()).ok())
            .unwrap_or_default();
        Some(SellListing {
            id,
            rev: v
                .get("_rev")
                .or_else(|| v.get("rev"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            seller_id: record_id_from_field(v, "seller_id")
                .or_else(|| {
                    v.get("seller_id")
                        .and_then(|x| x.as_str())
                        .map(|s| s.replace(':', "/"))
                })
                .unwrap_or_default(),
            status: v
                .get("status")
                .and_then(|x| x.as_str())
                .unwrap_or(listing_status::DRAFT_CREATED)
                .to_string(),
            created_at: parse_dt("created_at").unwrap_or_else(Utc::now),
            updated_at: parse_dt("updated_at").unwrap_or_else(Utc::now),
            expires_at: parse_dt("expires_at").unwrap_or_else(|| Utc::now() + Duration::hours(24)),
            title: v.get("title").and_then(|x| x.as_str()).map(String::from),
            description: v
                .get("description")
                .and_then(|x| x.as_str())
                .map(String::from),
            condition_notes: v
                .get("condition_notes")
                .and_then(|x| x.as_str())
                .map(String::from),
            condition: v.get("condition").and_then(|x| x.as_str()).map(String::from),
            price_cents: v.get("price_cents").and_then(|x| x.as_i64()),
            currency: v.get("currency").and_then(|x| x.as_str()).map(String::from),
            shipping_notes: v
                .get("shipping_notes")
                .and_then(|x| x.as_str())
                .map(String::from),
            bgg_id: v.get("bgg_id").and_then(|x| x.as_i64()).map(|n| n as i32),
            game_name: v.get("game_name").and_then(|x| x.as_str()).map(String::from),
            edition_notes: v
                .get("edition_notes")
                .and_then(|x| x.as_str())
                .map(String::from),
            missing_components: v
                .get("missing_components")
                .and_then(|x| serde_json::from_value(x.clone()).ok())
                .unwrap_or_default(),
            ai_confidence: v.get("ai_confidence").and_then(|x| x.as_f64()),
            ai_questions: v
                .get("ai_questions")
                .and_then(|x| serde_json::from_value(x.clone()).ok())
                .unwrap_or_default(),
            ai_warnings: v
                .get("ai_warnings")
                .and_then(|x| serde_json::from_value(x.clone()).ok())
                .unwrap_or_default(),
            bgg_listing_url: v
                .get("bgg_listing_url")
                .and_then(|x| x.as_str())
                .map(String::from),
            checkpoint_approvals: approvals,
            photo_count: v
                .get("photo_count")
                .and_then(|x| x.as_u64())
                .unwrap_or(0) as u32,
        })
    }

    fn value_to_photo(v: &serde_json::Value) -> Option<SellListingPhoto> {
        let id = record_id_from_row(v, None)?;
        let parse_dt = |k: &str| {
            v.get(k)
                .and_then(|x| x.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
        };
        Some(SellListingPhoto {
            id,
            listing_id: record_id_from_field(v, "listing_id")
                .or_else(|| {
                    v.get("listing_id")
                        .and_then(|x| x.as_str())
                        .map(|s| s.replace(':', "/"))
                })
                .unwrap_or_default(),
            sort_order: v
                .get("sort_order")
                .and_then(|x| x.as_u64())
                .unwrap_or(0) as u32,
            content_type: v
                .get("content_type")
                .and_then(|x| x.as_str())
                .unwrap_or("image/jpeg")
                .to_string(),
            size_bytes: v
                .get("size_bytes")
                .and_then(|x| x.as_u64())
                .unwrap_or(0),
            created_at: parse_dt("created_at").unwrap_or_else(Utc::now),
        })
    }

    pub fn enrich_dto(&self, mut dto: SellListingDto, include_photos: bool) -> SellListingDto {
        let key = Self::listing_key(&dto.id);
        if include_photos && !key.is_empty() {
            // photos filled by caller when needed
        }
        if !key.is_empty() {
            dto.photos.iter_mut().for_each(|p| {
                let pk = Self::photo_key(&p.id);
                p.preview_url = Some(format!(
                    "/api/sell/listings/{}/photos/{pk}",
                    key
                ));
            });
        }
        dto
    }

    pub async fn create_listing(&self, seller_id: &str) -> Result<SellListingDto, String> {
        let seller_key = record_id_to_key(seller_id, "player");
        if seller_key.is_empty() {
            return Err("invalid seller".to_string());
        }
        let listing_key = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires = now + Duration::hours(ttl_hours());
        let sql = self.query_with_scope(
            "CREATE type::record('sell_listing', $key) CONTENT {\
             seller_id: type::record('player', $seller_key),\
             status: 'draft_created',\
             created_at: type::datetime($created_at),\
             updated_at: type::datetime($updated_at),\
             expires_at: type::datetime($expires_at),\
             photo_count: 0,\
             missing_components: [],\
             ai_questions: [],\
             ai_warnings: [],\
             checkpoint_approvals: []\
             }",
        );
        let idx = self.scope_result_index();
        let mut res = self
            .db
            .query(&sql)
            .bind(("key", listing_key.clone()))
            .bind(("seller_key", seller_key))
            .bind(("created_at", now.to_rfc3339()))
            .bind(("updated_at", now.to_rfc3339()))
            .bind(("expires_at", expires.to_rfc3339()))
            .await
            .map_err(|e| format!("create listing: {e}"))?;
        let rows: Vec<serde_json::Value> = res.take(idx).map_err(|e| e.to_string())?;
        let listing = rows
            .first()
            .and_then(Self::value_to_listing)
            .ok_or_else(|| "create returned no row".to_string())?;
        Ok(SellListingDto::from(&listing))
    }

    pub async fn find_by_id(&self, listing_id: &str) -> Option<SellListing> {
        let key = Self::listing_key(listing_id);
        if key.is_empty() {
            return None;
        }
        let sql = self
            .query_with_scope("SELECT * FROM type::record('sell_listing', $key) LIMIT 1");
        let idx = self.scope_result_index();
        let mut res = self
            .db
            .query(&sql)
            .bind(("key", key))
            .await
            .ok()?;
        let rows: Vec<serde_json::Value> = res.take(idx).ok()?;
        rows.first().and_then(Self::value_to_listing)
    }

    pub async fn list_by_seller(&self, seller_id: &str) -> Vec<SellListingDto> {
        let seller_key = record_id_to_key(seller_id, "player");
        if seller_key.is_empty() {
            return Vec::new();
        }
        let sql = self.query_with_scope(
            "SELECT * FROM sell_listing WHERE seller_id = type::record('player', $seller_key) ORDER BY created_at DESC",
        );
        let idx = self.scope_result_index();
        let mut res = match self
            .db
            .query(&sql)
            .bind(("seller_key", seller_key))
            .await
        {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let rows: Vec<serde_json::Value> = res.take(idx).unwrap_or_default();
        rows.iter()
            .filter_map(Self::value_to_listing)
            .map(|l| SellListingDto::from(&l))
            .collect()
    }

    pub async fn list_photos(&self, listing_id: &str) -> Vec<SellListingPhoto> {
        let listing_key = Self::listing_key(listing_id);
        if listing_key.is_empty() {
            return Vec::new();
        }
        let sql = self.query_with_scope(
            "SELECT * FROM sell_listing_photo WHERE listing_id = type::record('sell_listing', $listing_key) ORDER BY sort_order ASC",
        );
        let idx = self.scope_result_index();
        let mut res = match self
            .db
            .query(&sql)
            .bind(("listing_key", listing_key))
            .await
        {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let rows: Vec<serde_json::Value> = res.take(idx).unwrap_or_default();
        rows.iter()
            .filter_map(Self::value_to_photo)
            .collect()
    }

    pub async fn add_photo(
        &self,
        listing_id: &str,
        content_type: &str,
        data: &[u8],
    ) -> Result<SellListingPhotoDto, String> {
        let listing = self
            .find_by_id(listing_id)
            .await
            .ok_or_else(|| "listing not found".to_string())?;
        if listing.status == listing_status::CANCELLED
            || listing.status == listing_status::SUBMITTED
        {
            return Err("listing is closed".to_string());
        }
        if listing.photo_count >= max_photo_count() {
            return Err(format!("maximum {} photos per listing", max_photo_count()));
        }
        let listing_key = Self::listing_key(listing_id);
        let photo_key = Uuid::new_v4().to_string();
        write_photo_atomic(&listing_key, &photo_key, data)?;
        let now = Utc::now();
        let sort_order = listing.photo_count;
        let sql = self.query_with_scope(
            "CREATE type::record('sell_listing_photo', $photo_key) CONTENT {\
             listing_id: type::record('sell_listing', $listing_key),\
             sort_order: $sort_order,\
             content_type: $content_type,\
             size_bytes: $size_bytes,\
             created_at: type::datetime($created_at)\
             }",
        );
        let idx = self.scope_result_index();
        let mut res = self
            .db
            .query(&sql)
            .bind(("photo_key", photo_key.clone()))
            .bind(("listing_key", listing_key.clone()))
            .bind(("sort_order", sort_order))
            .bind(("content_type", content_type.to_string()))
            .bind(("size_bytes", data.len() as u64))
            .bind(("created_at", now.to_rfc3339()))
            .await
            .map_err(|e| {
                delete_photo_file(&listing_key, &photo_key);
                format!("create photo record: {e}")
            })?;
        let rows: Vec<serde_json::Value> = res.take(idx).map_err(|e| e.to_string())?;
        let photo = rows
            .first()
            .and_then(Self::value_to_photo)
            .ok_or_else(|| "photo create returned no row".to_string())?;

        let new_status = if listing.status == listing_status::DRAFT_CREATED {
            listing_status::PHOTOS_UPLOADED
        } else {
            listing.status.as_str()
        };
        self.update_listing_fields(
            &listing_key,
            &serde_json::json!({
                "photo_count": listing.photo_count + 1,
                "status": new_status,
                "updated_at": now.to_rfc3339(),
            }),
        )
        .await?;

        let mut dto = SellListingPhotoDto::from(&photo);
        dto.preview_url = Some(format!(
            "/api/sell/listings/{listing_key}/photos/{photo_key}"
        ));
        Ok(dto)
    }

    pub async fn delete_photo(
        &self,
        listing_id: &str,
        photo_id: &str,
    ) -> Result<(), String> {
        let listing_key = Self::listing_key(listing_id);
        let photo_key = Self::photo_key(photo_id);
        if listing_key.is_empty() || photo_key.is_empty() {
            return Err("invalid id".to_string());
        }
        delete_photo_file(&listing_key, &photo_key);
        let sql = self.query_with_scope(
            "DELETE type::record('sell_listing_photo', $photo_key)",
        );
        let idx = self.scope_result_index();
        let _ = self
            .db
            .query(&sql)
            .bind(("photo_key", photo_key))
            .await
            .map_err(|e| e.to_string())?
            .take::<Vec<serde_json::Value>>(idx);

        let photos = self.list_photos(listing_id).await;
        self.update_listing_fields(
            &listing_key,
            &serde_json::json!({
                "photo_count": photos.len(),
                "updated_at": Utc::now().to_rfc3339(),
            }),
        )
        .await
    }

    async fn update_listing_fields(
        &self,
        listing_key: &str,
        fields: &serde_json::Value,
    ) -> Result<(), String> {
        let sql = self.query_with_scope(
            "UPDATE type::record('sell_listing', $key) MERGE $fields",
        );
        let idx = self.scope_result_index();
        let _ = self
            .db
            .query(&sql)
            .bind(("key", listing_key.to_string()))
            .bind(("fields", fields.clone()))
            .await
            .map_err(|e| e.to_string())?
            .take::<Vec<serde_json::Value>>(idx);
        Ok(())
    }

    pub async fn approve_checkpoint(
        &self,
        listing_id: &str,
        checkpoint_name: &str,
        approver_id: &str,
    ) -> Result<SellListingDto, String> {
        let listing = self
            .find_by_id(listing_id)
            .await
            .ok_or_else(|| "listing not found".to_string())?;
        if listing.has_checkpoint(checkpoint_name) {
            return Err("checkpoint already approved".to_string());
        }

        self.validate_checkpoint_prereqs(&listing, checkpoint_name)?;

        let new_status = status_after_checkpoint(checkpoint_name)
            .ok_or_else(|| "unknown checkpoint".to_string())?;
        let mut approvals = listing.checkpoint_approvals.clone();
        approvals.push(CheckpointApproval {
            checkpoint: checkpoint_name.to_string(),
            approved_at: Utc::now(),
            approved_by: approver_id.to_string(),
        });
        let listing_key = Self::listing_key(listing_id);
        self.update_listing_fields(
            &listing_key,
            &serde_json::json!({
                "status": new_status,
                "checkpoint_approvals": approvals,
                "updated_at": Utc::now().to_rfc3339(),
            }),
        )
        .await?;
        self.find_by_id(listing_id)
            .await
            .map(|l| SellListingDto::from(&l))
            .ok_or_else(|| "listing not found after update".to_string())
    }

    fn validate_checkpoint_prereqs(
        &self,
        listing: &SellListing,
        checkpoint_name: &str,
    ) -> Result<(), String> {
        match checkpoint_name {
            checkpoint::PHOTOS => {
                if listing.photo_count == 0 {
                    return Err("upload at least one photo".to_string());
                }
            }
            checkpoint::LISTING => {
                if listing.bgg_id.is_none() {
                    return Err("select a BGG game first".to_string());
                }
                if listing.price_cents.is_none() {
                    return Err("set a price first".to_string());
                }
            }
            checkpoint::AI_REVIEW => {
                if listing.status != listing_status::AI_DRAFT {
                    return Err("run AI extraction first".to_string());
                }
            }
            checkpoint::BGG_MATCH => {
                if !listing.has_checkpoint(checkpoint::AI_REVIEW) {
                    return Err("approve AI review first".to_string());
                }
                if listing.bgg_id.is_none() {
                    return Err("select a BGG game first".to_string());
                }
            }
            checkpoint::MARKETPLACE => {
                if listing.price_cents.is_none() {
                    return Err("set a price first".to_string());
                }
            }
            checkpoint::AUTOMATION => {
                if listing.status != listing_status::AUTOMATION_READY
                    && listing.status != listing_status::BGG_PREVIEW
                {
                    return Err("complete marketplace review first".to_string());
                }
            }
            _ => return Err("unknown checkpoint".to_string()),
        }
        Ok(())
    }

    pub async fn run_extraction(&self, listing_id: &str) -> Result<(SellListingDto, Option<shared::dto::sell_listing::AiClarifyDto>), String> {
        let listing = self
            .find_by_id(listing_id)
            .await
            .ok_or_else(|| "listing not found".to_string())?;
        if !listing.has_checkpoint(checkpoint::PHOTOS) {
            return Err("approve photos checkpoint first".to_string());
        }
        let listing_key = Self::listing_key(listing_id);
        let photos_meta = self.list_photos(listing_id).await;
        let mut photo_data: Vec<(String, Vec<u8>)> = Vec::new();
        for p in &photos_meta {
            let pk = Self::photo_key(&p.id);
            if let Some((bytes, _)) =
                read_photo_variant(&listing_key, &pk, SellPhotoVariant::Detail)
            {
                photo_data.push((p.content_type.clone(), bytes));
            }
        }
        let extracted = ai_extraction::extract_from_photos(&photo_data).await?;
        let clarify = extracted.clarify.clone();
        self.update_listing_fields(
            &listing_key,
            &serde_json::json!({
                "status": listing_status::AI_DRAFT,
                "title": extracted.title,
                "description": extracted.description,
                "condition_notes": extracted.condition_notes,
                "game_name": extracted.game_name,
                "edition_notes": extracted.edition_notes,
                "missing_components": extracted.missing_components,
                "ai_confidence": extracted.confidence,
                "ai_questions": extracted.questions,
                "ai_warnings": extracted.warnings,
                "updated_at": Utc::now().to_rfc3339(),
            }),
        )
        .await?;
        let dto = self
            .find_by_id(listing_id)
            .await
            .map(|l| SellListingDto::from(&l))
            .ok_or_else(|| "listing not found".to_string())?;
        Ok((dto, clarify))
    }

    pub async fn update_draft(
        &self,
        listing_id: &str,
        req: UpdateSellListingDraftRequest,
    ) -> Result<SellListingDto, String> {
        let listing_key = Self::listing_key(listing_id);
        let mut fields = serde_json::json!({ "updated_at": Utc::now().to_rfc3339() });
        let obj = fields.as_object_mut().unwrap();
        if let Some(v) = req.title {
            obj.insert("title".into(), v.into());
        }
        if let Some(v) = req.description {
            obj.insert("description".into(), v.into());
        }
        if let Some(v) = req.condition_notes {
            obj.insert("condition_notes".into(), v.into());
        }
        if let Some(v) = req.condition {
            obj.insert("condition".into(), v.into());
        }
        if let Some(v) = req.price_cents {
            obj.insert("price_cents".into(), v.into());
        }
        if let Some(v) = req.currency {
            obj.insert("currency".into(), v.into());
        }
        if let Some(v) = req.shipping_notes {
            obj.insert("shipping_notes".into(), v.into());
        }
        if let Some(v) = req.game_name {
            obj.insert("game_name".into(), v.into());
        }
        if let Some(v) = req.edition_notes {
            obj.insert("edition_notes".into(), v.into());
        }
        if let Some(v) = req.missing_components {
            obj.insert("missing_components".into(), serde_json::json!(v));
        }
        self.update_listing_fields(&listing_key, &fields).await?;
        self.find_by_id(listing_id)
            .await
            .map(|l| SellListingDto::from(&l))
            .ok_or_else(|| "listing not found".to_string())
    }

    pub async fn set_bgg_match(
        &self,
        listing_id: &str,
        bgg_id: i32,
        game_name: &str,
    ) -> Result<SellListingDto, String> {
        let listing_key = Self::listing_key(listing_id);
        self.update_listing_fields(
            &listing_key,
            &serde_json::json!({
                "bgg_id": bgg_id,
                "game_name": game_name,
                "updated_at": Utc::now().to_rfc3339(),
            }),
        )
        .await?;
        self.find_by_id(listing_id)
            .await
            .map(|l| SellListingDto::from(&l))
            .ok_or_else(|| "listing not found".to_string())
    }

    pub async fn build_export(
        &self,
        listing_id: &str,
        prefs: &shared::dto::sell_preferences::SellPreferencesDto,
    ) -> Result<BggExportPayload, String> {
        let listing = self
            .find_by_id(listing_id)
            .await
            .ok_or_else(|| "listing not found".to_string())?;
        if listing.bgg_id.is_none() {
            return Err("select a BGG game first".to_string());
        }
        if listing.price_cents.is_none() {
            return Err("set a price first".to_string());
        }
        let listing_key = Self::listing_key(listing_id);
        let photos = self.list_photos(listing_id).await;
        let photo_paths: Vec<String> = photos
            .iter()
            .map(|p| {
                let pk = Self::photo_key(&p.id);
                crate::sell::image::photo_path(&listing_key, &pk)
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        let game_name = listing.game_name.clone().unwrap_or_default();
        let condition = listing
            .condition
            .clone()
            .unwrap_or_else(|| prefs.condition.clone());
        let currency = listing
            .currency
            .clone()
            .unwrap_or_else(|| prefs.currency.clone());

        let mut description_parts = Vec::new();
        if let Some(d) = &listing.description {
            if !d.is_empty() {
                description_parts.push(d.clone());
            }
        }
        if !prefs.seller_notes.is_empty() {
            description_parts.push(prefs.seller_notes.clone());
        }
        if let Some(n) = &listing.condition_notes {
            if !n.is_empty() {
                description_parts.push(format!("Condition notes: {n}"));
            }
        }
        if !listing.missing_components.is_empty() {
            description_parts.push(format!(
                "Missing: {}",
                listing.missing_components.join(", ")
            ));
        }

        Ok(BggExportPayload {
            listing_id: listing.id.clone(),
            title: listing
                .title
                .clone()
                .filter(|t| !t.is_empty())
                .unwrap_or(game_name.clone()),
            description: description_parts.join("\n\n"),
            condition,
            condition_notes: listing.condition_notes.unwrap_or_default(),
            price_cents: listing.price_cents.unwrap_or(0),
            currency,
            shipping_notes: listing
                .shipping_notes
                .clone()
                .unwrap_or_else(|| prefs.ship_to.clone()),
            bgg_id: listing.bgg_id.unwrap_or(0),
            game_name,
            edition_notes: listing.edition_notes.unwrap_or_default(),
            missing_components: listing.missing_components.clone(),
            payment_paypal: prefs.payment_paypal,
            payment_other: prefs.payment_other,
            item_location: prefs.item_location.clone(),
            ship_to: prefs.ship_to.clone(),
            seller_notes: prefs.seller_notes.clone(),
            photo_paths,
        })
    }

    pub async fn record_automation_result(
        &self,
        listing_id: &str,
        success: bool,
        bgg_listing_url: Option<String>,
        error_message: Option<String>,
        submitted_on_bgg: bool,
    ) -> Result<SellListingDto, String> {
        let listing_key = Self::listing_key(listing_id);
        let status = if submitted_on_bgg {
            listing_status::SUBMITTED
        } else if success {
            listing_status::BGG_PREVIEW
        } else {
            listing_status::AUTOMATION_READY
        };
        let mut fields = serde_json::json!({
            "status": status,
            "updated_at": Utc::now().to_rfc3339(),
        });
        if let Some(url) = bgg_listing_url {
            fields["bgg_listing_url"] = url.into();
        }
        if let Some(err) = error_message {
            fields["ai_warnings"] = serde_json::json!([err]);
        }
        self.update_listing_fields(&listing_key, &fields).await?;
        if submitted_on_bgg {
            let photos = self.list_photos(listing_id).await;
            let keys: Vec<String> = photos
                .iter()
                .map(|p| Self::photo_key(&p.id))
                .collect();
            delete_all_listing_photos(&listing_key, &keys);
        }
        self.find_by_id(listing_id)
            .await
            .map(|l| SellListingDto::from(&l))
            .ok_or_else(|| "listing not found".to_string())
    }

    pub async fn cancel_listing(&self, listing_id: &str) -> Result<(), String> {
        let listing_key = Self::listing_key(listing_id);
        let photos = self.list_photos(listing_id).await;
        let keys: Vec<String> = photos
            .iter()
            .map(|p| Self::photo_key(&p.id))
            .collect();
        delete_all_listing_photos(&listing_key, &keys);
        let sql = self.query_with_scope(
            "DELETE sell_listing_photo WHERE listing_id = type::record('sell_listing', $listing_key)",
        );
        let idx = self.scope_result_index();
        let _ = self
            .db
            .query(&sql)
            .bind(("listing_key", listing_key.clone()))
            .await
            .map_err(|e| e.to_string())?
            .take::<Vec<serde_json::Value>>(idx);
        self.update_listing_fields(
            &listing_key,
            &serde_json::json!({
                "status": listing_status::CANCELLED,
                "photo_count": 0,
                "updated_at": Utc::now().to_rfc3339(),
            }),
        )
        .await
    }

    pub async fn purge_expired_listings(&self) -> Result<usize, String> {
        let now = Utc::now().to_rfc3339();
        let sql = self.query_with_scope(
            "SELECT id FROM sell_listing WHERE expires_at < type::datetime($now) AND status != 'submitted' AND status != 'cancelled'",
        );
        let idx = self.scope_result_index();
        let mut res = self
            .db
            .query(&sql)
            .bind(("now", now))
            .await
            .map_err(|e| e.to_string())?;
        let rows: Vec<serde_json::Value> = res.take(idx).unwrap_or_default();
        let mut count = 0usize;
        for row in rows {
            if let Some(id) = record_id_from_row(&row, None) {
                let _ = self.cancel_listing(&id).await;
                count += 1;
            }
        }
        Ok(count)
    }

    pub fn read_photo_bytes(
        &self,
        listing_id: &str,
        photo_id: &str,
        variant: SellPhotoVariant,
    ) -> Option<(Vec<u8>, String)> {
        let listing_key = Self::listing_key(listing_id);
        let photo_key = Self::photo_key(photo_id);
        read_photo_variant(&listing_key, &photo_key, variant)
            .map(|(bytes, mime)| (bytes, mime.to_string()))
    }
}

#[async_trait]
pub trait SellListingRepository: Send + Sync {
    async fn create_listing(&self, seller_id: &str) -> Result<SellListingDto, String>;
}

#[async_trait]
impl SellListingRepository for SellListingRepositoryImpl {
    async fn create_listing(&self, seller_id: &str) -> Result<SellListingDto, String> {
        SellListingRepositoryImpl::create_listing(self, seller_id).await
    }
}
