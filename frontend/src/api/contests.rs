use crate::api::api_url;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ContestSearchItem {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub start: String,
    pub stop: String,
    pub venue: Option<serde_json::Value>,
    pub games: Vec<serde_json::Value>,
    pub outcomes: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContestSearchResponse {
    pub items: Vec<ContestSearchItem>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

pub async fn search_contests(params: &[(&str, String)]) -> Result<ContestSearchResponse, String> {
    // Build URL-encoded query string to safely handle spaces and special characters
    let mut qs = String::new();
    if !params.is_empty() {
        qs.push('?');
        for (i, (k, v)) in params.iter().enumerate() {
            if i > 0 {
                qs.push('&');
            }
            let encoded_key = js_sys::encode_uri_component(k);
            let encoded_val = js_sys::encode_uri_component(v);
            qs.push_str(&encoded_key.as_string().unwrap_or_else(|| k.to_string()));
            qs.push('=');
            qs.push_str(&encoded_val.as_string().unwrap_or_else(|| v.to_string()));
        }
    }
    let url = format!("{}{}", api_url("/api/contests/search"), qs);
    let resp = authenticated_get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json::<ContestSearchResponse>()
        .await
        .map_err(|e| e.to_string())
}

use crate::api::utils::{
    authenticated_delete, authenticated_get, authenticated_post, authenticated_put,
};
use log::debug;
use shared::{ContestDto, ErrorResponse};

pub async fn submit_contest(contest: ContestDto) -> Result<ContestDto, String> {
    debug!("Submitting contest with ID: {}", contest.id);
    gloo::console::log!("🌐 API: submit_contest function called");
    gloo::console::log!("🌐 API: Contest ID:", &contest.id);
    gloo::console::log!("🌐 API: Contest name:", &contest.name);
    gloo::console::log!("🌐 API: Contest timezone:", &contest.venue.timezone);

    let req = authenticated_post(&api_url("/api/contests"));
    gloo::console::log!("API: Created authenticated request");

    gloo::console::log!("API: Serializing contest to JSON");
    let response = match req.json(&contest).map_err(|e| e.to_string())?.send().await {
        Ok(resp) => {
            gloo::console::log!("API: Request sent successfully");
            gloo::console::log!("API: Response status code:", resp.status());
            resp
        }
        Err(e) => {
            let err_msg = format!("Failed to send contest: {}", e);
            gloo::console::error!("API:", &err_msg);
            return Err(err_msg);
        }
    };

    gloo::console::log!("API: Response status:", response.status());

    if !response.ok() {
        gloo::console::error!("API: Response not OK, status:", response.status());
        let error = match response.json::<ErrorResponse>().await {
            Ok(err) => {
                gloo::console::error!("API: Error response:", &err.error);
                err.error
            }
            Err(_) => {
                let msg = "Unknown error occurred".to_string();
                gloo::console::error!("API:", &msg);
                msg
            }
        };
        return Err(error);
    }

    gloo::console::log!("API: Parsing response body");
    let saved_contest = match response.json::<ContestDto>().await {
        Ok(contest) => {
            gloo::console::log!("API: Response parsed successfully");
            contest
        }
        Err(e) => {
            let err_msg = format!("Failed to parse contest response: {}", e);
            gloo::console::error!("API:", &err_msg);
            return Err(err_msg);
        }
    };

    gloo::console::log!(
        "API: Successfully submitted contest:",
        saved_contest.name.clone()
    );
    debug!("Successfully submitted contest: {}", saved_contest.name);
    Ok(saved_contest)
}

pub async fn get_contest_by_id(id: &str) -> Result<ContestDto, String> {
    debug!("Fetching contest with ID: {}", id);

    let response = authenticated_get(&format!("{}/{}", api_url("/api/contests"), id))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch contest: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let contest = response
        .json::<ContestDto>()
        .await
        .map_err(|e| format!("Failed to parse contest response: {}", e))?;

    debug!("Successfully fetched contest with ID: {}", contest.id);
    Ok(contest)
}

pub async fn list_contests() -> Result<Vec<ContestDto>, String> {
    debug!("Fetching all contests");

    let response = authenticated_get(&api_url("/api/contests"))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch contests: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let contests = response
        .json::<Vec<ContestDto>>()
        .await
        .map_err(|e| format!("Failed to parse contests response: {}", e))?;

    debug!("Successfully fetched {} contests", contests.len());
    Ok(contests)
}

pub async fn update_contest(id: &str, contest: ContestDto) -> Result<ContestDto, String> {
    debug!("Updating contest with ID: {}", contest.id);

    let response = authenticated_put(&format!("{}/{}", api_url("/api/contests"), id))
        .json(&contest)
        .map_err(|e| format!("Failed to serialize contest update: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send contest update: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    let updated_contest = response
        .json::<ContestDto>()
        .await
        .map_err(|e| format!("Failed to parse contest update response: {}", e))?;

    debug!(
        "Successfully updated contest with ID: {}",
        updated_contest.id
    );
    Ok(updated_contest)
}

pub async fn delete_contest(id: &str) -> Result<(), String> {
    debug!("Deleting contest with ID: {}", id);

    let response = authenticated_delete(&format!("{}/{}", api_url("/api/contests"), id))
        .send()
        .await
        .map_err(|e| format!("Failed to delete contest: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }

    debug!("Successfully deleted contest with ID: {}", id);
    Ok(())
}
