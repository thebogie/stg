//! BGG Playwright automation — local subprocess (dev) or Redis queue (production).

use shared::dto::sell_listing::BggExportPayload;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

use crate::sell::playwright_queue;

fn script_path() -> PathBuf {
    std::env::var("BGG_PLAYWRIGHT_SCRIPT").map_or_else(
        |_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tools/bgg-marketplace/fill-listing.mjs")
        },
        PathBuf::from,
    )
}

fn node_binary() -> String {
    std::env::var("NODE_BIN").unwrap_or_else(|_| "node".to_string())
}

/// Run BGG marketplace automation locally. Credentials via env only — never logged.
pub async fn run_bgg_automation_local(
    payload: &BggExportPayload,
    bgg_username: &str,
    bgg_password: &str,
) -> Result<String, String> {
    let script = script_path();
    if !script.exists() {
        return Err(format!(
            "Playwright script not found at {}",
            script.display()
        ));
    }

    let temp_dir = std::env::temp_dir().join(format!("stg_bgg_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    let json_path = temp_dir.join("listing.json");
    let json = serde_json::to_string_pretty(payload).map_err(|e| e.to_string())?;
    std::fs::write(&json_path, json).map_err(|e| e.to_string())?;

    let headless = std::env::var("BGG_HEADLESS").unwrap_or_else(|_| "1".to_string());
    let auto_submit = std::env::var("BGG_AUTO_SUBMIT").unwrap_or_else(|_| "0".to_string());

    let output = Command::new(node_binary())
        .arg(&script)
        .arg(&json_path)
        .env("BGG_USERNAME", bgg_username)
        .env("BGG_PASSWORD", bgg_password)
        .env("BGG_HEADLESS", headless)
        .env("BGG_AUTO_SUBMIT", auto_submit)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("failed to spawn Playwright: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(if stderr.is_empty() {
            format!("Playwright failed: {stdout}")
        } else {
            format!("Playwright failed: {stderr}")
        });
    }

    Ok(if stdout.trim().is_empty() {
        "BGG form filled — review listing on BoardGameGeek and submit.".to_string()
    } else {
        stdout.trim().to_string()
    })
}

/// Enqueue BGG automation for the Playwright worker container.
pub async fn enqueue_bgg_automation(
    redis: &redis::Client,
    listing_id: &str,
    player_id: &str,
    payload: &BggExportPayload,
    bgg_username: &str,
    bgg_password: &str,
) -> Result<String, String> {
    playwright_queue::enqueue_bgg_job(
        redis,
        listing_id,
        player_id,
        payload,
        bgg_username,
        bgg_password,
    )
    .await
}
