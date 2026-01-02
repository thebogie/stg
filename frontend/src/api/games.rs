use shared::{GameDto, ErrorResponse};
use log::debug;
use crate::api::api_url;
use crate::api::utils::{authenticated_get, authenticated_put, authenticated_delete};

pub async fn search_games(query: &str) -> Result<Vec<GameDto>, String> {
    debug!("Searching games with query: {}", query);
    
    let response = authenticated_get(&format!("{}?query={}", api_url("/api/games/db_search"), query))
        .send()
        .await
        .map_err(|e| format!("Failed to search games: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let games = response
        .json::<Vec<GameDto>>()
        .await
        .map_err(|e| format!("Failed to parse games response: {}", e))?;

    debug!("Successfully found {} games", games.len());
    Ok(games)
}

pub async fn get_all_games() -> Result<Vec<GameDto>, String> {
    debug!("Fetching all games");
    
    let response = authenticated_get(&api_url("/api/games"))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch games: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let games = response
        .json::<Vec<GameDto>>()
        .await
        .map_err(|e| format!("Failed to parse games response: {}", e))?;

    debug!("Successfully found {} games", games.len());
    Ok(games)
}

pub async fn get_game_by_id(id: &str) -> Result<GameDto, String> {
    debug!("Fetching game with ID: {}", id);
    
    let response = authenticated_get(&format!("{}/{}", api_url("/api/games"), id))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch game: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let game = response
        .json::<GameDto>()
        .await
        .map_err(|e| format!("Failed to parse game response: {}", e))?;

    debug!("Successfully fetched game: {}", game.name);
    Ok(game)
}

pub async fn update_game(id: &str, game: GameDto) -> Result<GameDto, String> {
    debug!("Updating game with ID: {}", id);
    // Normalize: backend route is /api/games/{id} where {id} should NOT contain a slash
    let id_param = if let Some(stripped) = id.strip_prefix("game/") { stripped } else { id };
    let url = format!("{}/{}", api_url("/api/games"), id_param);
    
    let response = authenticated_put(&url)
        .json(&game)
        .map_err(|e| format!("Failed to serialize game: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to update game: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let updated_game = response
        .json::<GameDto>()
        .await
        .map_err(|e| format!("Failed to parse game response: {}", e))?;

    debug!("Successfully updated game: {}", updated_game.name);
    Ok(updated_game)
}

pub async fn delete_game(id: &str) -> Result<(), String> {
    debug!("Deleting game with ID: {}", id);
    // Normalize: backend route is /api/games/{id} where {id} should NOT contain a slash
    let id_param = if let Some(stripped) = id.strip_prefix("game/") { stripped } else { id };
    let url = format!("{}/{}", api_url("/api/games"), id_param);
    
    let response = authenticated_delete(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to delete game: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    debug!("Successfully deleted game: {}", id);
    Ok(())
}

pub async fn get_game_analytics(game_id: &str) -> Result<serde_json::Value, String> {
    debug!("Fetching analytics for game: {}", game_id);
    
    let response = authenticated_get(&format!("{}/analytics/{}", api_url("/api/games"), game_id))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch game analytics: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let analytics = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse analytics response: {}", e))?;

    debug!("Successfully fetched analytics for game: {}", game_id);
    Ok(analytics)
}

pub async fn find_similar_games(game_id: &str) -> Result<Vec<GameDto>, String> {
    debug!("Finding similar games for: {}", game_id);
    
    let response = authenticated_get(&format!("{}/similar/{}", api_url("/api/games"), game_id))
        .send()
        .await
        .map_err(|e| format!("Failed to find similar games: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let games = response
        .json::<Vec<GameDto>>()
        .await
        .map_err(|e| format!("Failed to parse similar games response: {}", e))?;

    debug!("Successfully found {} similar games", games.len());
    Ok(games)
}

pub async fn merge_games(source_game_id: &str, target_game_id: &str) -> Result<GameDto, String> {
    debug!("Merging game {} into {}", source_game_id, target_game_id);
    
    let response = authenticated_put(&format!("{}/merge", api_url("/api/games")))
        .json(&serde_json::json!({
            "source_game_id": source_game_id,
            "target_game_id": target_game_id
        }))
        .map_err(|e| format!("Failed to serialize merge request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to merge games: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let merged_game = response
        .json::<GameDto>()
        .await
        .map_err(|e| format!("Failed to parse merge response: {}", e))?;

    debug!("Successfully merged games into: {}", merged_game.name);
    Ok(merged_game)
} 