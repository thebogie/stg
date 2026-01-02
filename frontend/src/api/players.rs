use gloo_net::http::Request;
use gloo_storage::Storage;
use shared::dto::player::PlayerDto;
use shared::dto::common::ErrorResponse;
use log::debug;
use crate::api::api_url;

pub async fn search_players(query: &str) -> Result<Vec<PlayerDto>, String> {
    debug!("Searching players with query: {}", query);

    let session_id = gloo_storage::LocalStorage::get::<String>("session_id").ok();
    let mut req = Request::get(&format!("{}?query={}", api_url("/api/players/search"), query));
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
