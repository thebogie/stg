//! Redis queue for Playwright browser automation jobs (BGG and future sites).

use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use shared::dto::sell_listing::BggExportPayload;
use shared::dto::sell_preferences::{
    playwright_job_status, playwright_job_type, PlaywrightJobStatusDto,
};

pub const QUEUE_KEY: &str = "playwright:queue";
const JOB_PREFIX: &str = "playwright:job:";
const STATUS_PREFIX: &str = "playwright:status:";
const FINALIZED_PREFIX: &str = "playwright:finalized:";

fn job_key(job_id: &str) -> String {
    format!("{JOB_PREFIX}{job_id}")
}

fn status_key(job_id: &str) -> String {
    format!("{STATUS_PREFIX}{job_id}")
}

fn finalized_key(job_id: &str) -> String {
    format!("{FINALIZED_PREFIX}{job_id}")
}

fn job_ttl_secs() -> u64 {
    std::env::var("PLAYWRIGHT_JOB_TTL_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(900)
}

fn status_ttl_secs() -> u64 {
    std::env::var("PLAYWRIGHT_STATUS_TTL_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaywrightJobSecrets {
    pub bgg_username: String,
    pub bgg_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaywrightJobRecord {
    pub job_id: String,
    pub job_type: String,
    pub listing_id: String,
    pub player_id: String,
    pub payload: BggExportPayload,
    pub secrets: PlaywrightJobSecrets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlaywrightStatusRecord {
    job_id: String,
    listing_id: String,
    player_id: String,
    job_type: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl From<PlaywrightStatusRecord> for PlaywrightJobStatusDto {
    fn from(r: PlaywrightStatusRecord) -> Self {
        PlaywrightJobStatusDto {
            job_id: r.job_id,
            listing_id: r.listing_id,
            job_type: r.job_type,
            status: r.status,
            message: r.message,
            error: r.error,
        }
    }
}

pub fn playwright_mode() -> String {
    std::env::var("PLAYWRIGHT_MODE").unwrap_or_else(|_| "local".to_string())
}

pub fn is_queue_mode() -> bool {
    playwright_mode().eq_ignore_ascii_case("queue")
}

pub async fn enqueue_bgg_job(
    redis: &redis::Client,
    listing_id: &str,
    player_id: &str,
    payload: &BggExportPayload,
    bgg_username: &str,
    bgg_password: &str,
) -> Result<String, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    let job = PlaywrightJobRecord {
        job_id: job_id.clone(),
        job_type: playwright_job_type::BGG_GEEKMARKET_FILL.to_string(),
        listing_id: listing_id.to_string(),
        player_id: player_id.to_string(),
        payload: payload.clone(),
        secrets: PlaywrightJobSecrets {
            bgg_username: bgg_username.to_string(),
            bgg_password: bgg_password.to_string(),
        },
    };
    let status = PlaywrightStatusRecord {
        job_id: job_id.clone(),
        listing_id: listing_id.to_string(),
        player_id: player_id.to_string(),
        job_type: playwright_job_type::BGG_GEEKMARKET_FILL.to_string(),
        status: playwright_job_status::QUEUED.to_string(),
        message: Some("BGG automation queued".to_string()),
        error: None,
    };

    let job_json = serde_json::to_string(&job).map_err(|e| e.to_string())?;
    let status_json = serde_json::to_string(&status).map_err(|e| e.to_string())?;
    let job_ttl = job_ttl_secs();
    let status_ttl = status_ttl_secs();

    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connection failed: {e}"))?;

    redis::pipe()
        .atomic()
        .set_ex(job_key(&job_id), job_json, job_ttl)
        .set_ex(status_key(&job_id), status_json, status_ttl)
        .lpush(QUEUE_KEY, &job_id)
        .query_async::<_, ()>(&mut conn)
        .await
        .map_err(|e| format!("failed to enqueue Playwright job: {e}"))?;

    Ok(job_id)
}

/// Enqueue a site smoke test job (Playwright health checks).
pub async fn enqueue_smoke_job(
    redis: &redis::Client,
    player_id: &str,
    base_url: &str,
) -> Result<String, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    let job = serde_json::json!({
        "job_id": job_id,
        "job_type": shared::dto::sell_preferences::playwright_job_type::SMOKE_STG,
        "listing_id": "smoke",
        "player_id": player_id,
        "payload": { "base_url": base_url },
        "secrets": { "bgg_username": "", "bgg_password": "" },
    });
    let status = PlaywrightStatusRecord {
        job_id: job_id.clone(),
        listing_id: "smoke".to_string(),
        player_id: player_id.to_string(),
        job_type: shared::dto::sell_preferences::playwright_job_type::SMOKE_STG.to_string(),
        status: playwright_job_status::QUEUED.to_string(),
        message: Some("STG smoke test queued".to_string()),
        error: None,
    };

    let job_json = serde_json::to_string(&job).map_err(|e| e.to_string())?;
    let status_json = serde_json::to_string(&status).map_err(|e| e.to_string())?;
    let job_ttl = job_ttl_secs();
    let status_ttl = status_ttl_secs();

    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connection failed: {e}"))?;

    redis::pipe()
        .atomic()
        .set_ex(job_key(&job_id), job_json, job_ttl)
        .set_ex(status_key(&job_id), status_json, status_ttl)
        .lpush(QUEUE_KEY, &job_id)
        .query_async::<_, ()>(&mut conn)
        .await
        .map_err(|e| format!("failed to enqueue smoke job: {e}"))?;

    Ok(job_id)
}

pub async fn get_job_status(
    redis: &redis::Client,
    job_id: &str,
    listing_id: &str,
    player_id: &str,
) -> Result<PlaywrightJobStatusDto, String> {
    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connection failed: {e}"))?;

    let raw: Option<String> = conn
        .get(status_key(job_id))
        .await
        .map_err(|e| format!("failed to read job status: {e}"))?;

    let Some(raw) = raw else {
        return Err("job not found or expired".to_string());
    };

    let record: PlaywrightStatusRecord =
        serde_json::from_str(&raw).map_err(|e| format!("invalid job status: {e}"))?;

    if record.listing_id != listing_id {
        return Err("job does not belong to this listing".to_string());
    }
    if record.player_id != player_id {
        return Err("forbidden".to_string());
    }

    Ok(record.into())
}

/// Mark listing automation result once when job reaches a terminal state.
pub async fn try_mark_finalized(redis: &redis::Client, job_id: &str) -> Result<bool, String> {
    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connection failed: {e}"))?;

    let key = finalized_key(job_id);
    let inserted: bool = redis::cmd("SET")
        .arg(&key)
        .arg("1")
        .arg("NX")
        .arg("EX")
        .arg(status_ttl_secs())
        .query_async(&mut conn)
        .await
        .map_err(|e| format!("failed to mark job finalized: {e}"))?;

    Ok(inserted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_record_roundtrip() {
        let job = PlaywrightJobRecord {
            job_id: "j1".to_string(),
            job_type: playwright_job_type::BGG_GEEKMARKET_FILL.to_string(),
            listing_id: "sell_listing/x".to_string(),
            player_id: "player/y".to_string(),
            payload: BggExportPayload {
                listing_id: "sell_listing/x".to_string(),
                title: "Test".to_string(),
                description: String::new(),
                condition: "good".to_string(),
                condition_notes: String::new(),
                price_cents: 1000,
                currency: "USD".to_string(),
                shipping_notes: String::new(),
                bgg_id: 1,
                game_name: "Test".to_string(),
                edition_notes: String::new(),
                missing_components: vec![],
                payment_paypal: true,
                payment_other: false,
                item_location: String::new(),
                ship_to: String::new(),
                seller_notes: String::new(),
                photo_paths: vec![],
            },
            secrets: PlaywrightJobSecrets {
                bgg_username: "u".to_string(),
                bgg_password: "p".to_string(),
            },
        };
        let json = serde_json::to_string(&job).unwrap();
        let back: PlaywrightJobRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.job_id, "j1");
        assert_eq!(back.secrets.bgg_username, "u");
    }
}
