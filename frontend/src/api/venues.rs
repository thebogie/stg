use crate::api::api_url;
use crate::api::utils::{authenticated_get, authenticated_post, authenticated_put};
use log::debug;
use shared::{ErrorResponse, VenueDto};

pub async fn get_all_venues() -> Result<Vec<VenueDto>, String> {
    debug!("Fetching all venues");

    let response = authenticated_get(&api_url("/api/venues"))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch venues: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let venues = response
        .json::<Vec<VenueDto>>()
        .await
        .map_err(|e| format!("Failed to parse venues response: {}", e))?;

    debug!("Successfully found {} venues", venues.len());
    Ok(venues)
}

pub async fn search_venues(query: &str) -> Result<Vec<VenueDto>, String> {
    debug!("Searching venues with query: {}", query);

    let response = authenticated_get(&format!(
        "{}?query={}",
        api_url("/api/venues/db_search"),
        query
    ))
    .send()
    .await
    .map_err(|e| format!("Failed to search venues: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let venues = response
        .json::<Vec<VenueDto>>()
        .await
        .map_err(|e| format!("Failed to parse venues response: {}", e))?;

    debug!("Successfully found {} venues", venues.len());
    Ok(venues)
}

pub async fn search_venues_for_create(query: &str) -> Result<Vec<VenueDto>, String> {
    debug!("Searching venues for create with query: {}", query);

    let response = authenticated_get(&format!(
        "{}?query={}",
        api_url("/api/venues/create_search"),
        query
    ))
    .send()
    .await
    .map_err(|e| format!("Failed to search venues for create: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let venues = response
        .json::<Vec<VenueDto>>()
        .await
        .map_err(|e| format!("Failed to parse venues response: {}", e))?;

    debug!("Successfully found {} venues for create", venues.len());
    Ok(venues)
}

pub async fn get_venue_by_id(id: &str) -> Result<VenueDto, String> {
    debug!("Fetching venue with ID: {}", id);
    // Normalize: backend route is /api/venues/{id} where {id} should NOT contain a slash
    let id_param = if let Some(stripped) = id.strip_prefix("venue/") {
        stripped
    } else {
        id
    };
    let url = format!("{}/{}", api_url("/api/venues"), id_param);
    let response = authenticated_get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch venue: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let venue = response
        .json::<VenueDto>()
        .await
        .map_err(|e| format!("Failed to parse venue response: {}", e))?;

    debug!("Successfully fetched venue: {}", venue.display_name);
    Ok(venue)
}

pub async fn create_venue(venue: VenueDto) -> Result<VenueDto, String> {
    debug!("Creating venue: {}", venue.display_name);

    let response = authenticated_post(&api_url("/api/venues"))
        .json(&venue)
        .map_err(|e| format!("Failed to serialize venue: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to create venue: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let created_venue = response
        .json::<VenueDto>()
        .await
        .map_err(|e| format!("Failed to parse venue response: {}", e))?;

    debug!("Successfully created venue: {}", created_venue.display_name);
    Ok(created_venue)
}

pub async fn update_venue(id: &str, venue: VenueDto) -> Result<VenueDto, String> {
    debug!("Updating venue with ID: {}", id);
    // Normalize: backend route is /api/venues/{id} where {id} should NOT contain a slash
    let id_param = if let Some(stripped) = id.strip_prefix("venue/") {
        stripped
    } else {
        id
    };
    let url = format!("{}/{}", api_url("/api/venues"), id_param);

    let response = authenticated_put(&url)
        .json(&venue)
        .map_err(|e| format!("Failed to serialize venue: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to update venue: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let updated_venue = response
        .json::<VenueDto>()
        .await
        .map_err(|e| format!("Failed to parse venue response: {}", e))?;

    debug!("Successfully updated venue: {}", updated_venue.display_name);
    Ok(updated_venue)
}
