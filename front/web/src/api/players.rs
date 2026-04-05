use crate::api::api_url;
use gloo_net::http::Request;
use gloo_storage::Storage;
use log::debug;
use shared::dto::common::ErrorResponse;
use shared::dto::player::PlayerDto;

/// Search or browse players. Pass an empty `query` to list players alphabetically by handle (see API `limit`).
pub async fn search_players(query: &str, limit: u32) -> Result<Vec<PlayerDto>, String> {
    debug!(
        "Searching players with query: {:?}, limit: {}",
        query, limit
    );

    let session_id = gloo_storage::LocalStorage::get::<String>("session_id").ok();
    let q_enc = urlencoding::encode(query);
    let mut req = Request::get(&format!(
        "{}?query={}&limit={}",
        api_url("/api/players/search"),
        q_enc,
        limit
    ));
    if let Some(sid) = session_id {
        req = req.header("Authorization", &format!("Bearer {}", sid));
    }

    let response = req
        .send()
        .await
        .map_err(|e| format!("Failed to search players: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let players = response
        .json::<Vec<PlayerDto>>()
        .await
        .map_err(|e| format!("Failed to parse players response: {}", e))?;

    debug!("Successfully found {} players", players.len());
    Ok(players)
}
