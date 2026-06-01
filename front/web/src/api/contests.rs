use crate::api::api_url;
use serde::{Deserialize, Serialize};

/// Extract a stable contest key from various id formats.
///
/// Examples:
/// - `contest/9d85...` -> `9d85...`
/// - `contest:9d85...` -> `9d85...`
/// - `contest:\`9d85...\`` -> `9d85...`
pub fn contest_key_from_any(id: &str) -> String {
    let cleaned = id.trim().replace('`', "");

    // Prefer known prefixes
    let without_prefix = cleaned
        .strip_prefix("contest/")
        .or_else(|| cleaned.strip_prefix("contest:"))
        .unwrap_or(&cleaned);

    // If something like `table/key` slipped in, keep only the key
    without_prefix
        .rsplit('/')
        .next()
        .unwrap_or(without_prefix)
        .to_string()
}

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
    #[serde(default)]
    pub has_image: bool,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub image_detail_url: Option<String>,
}

/// Max raw upload before server resize (must match backend `CONTEST_IMAGE_UPLOAD_MAX_BYTES`).
pub const MAX_CONTEST_IMAGE_UPLOAD_BYTES: usize = 8 * 1024 * 1024;

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
use wasm_bindgen::JsCast;

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
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        let error = serde_json::from_str::<ErrorResponse>(&body)
            .ok()
            .map(|err| err.error)
            .unwrap_or_else(|| {
                gloo::console::error!("API: Non-JSON error body:", &body);
                if body.is_empty() {
                    format!("Request failed (HTTP {})", response.status())
                } else {
                    body
                }
            });
        gloo::console::error!("API: Error response:", &error);
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

/// Contests awaiting moderator approval (admin API).
pub async fn list_pending_contests() -> Result<Vec<ContestDto>, String> {
    let url = api_url("/api/contests/moderation/pending");
    let response = authenticated_get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to list pending contests: {}", e))?;
    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }
    response
        .json::<Vec<ContestDto>>()
        .await
        .map_err(|e| format!("Failed to parse pending contests: {}", e))
}

pub async fn approve_contest(id: &str) -> Result<(), String> {
    let key = contest_key_from_any(id);
    let url = format!("{}/{}/approve", api_url("/api/contests"), key);
    let response = authenticated_post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to approve contest: {}", e))?;
    if !(response.ok() || response.status() == 204) {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }
    Ok(())
}

#[derive(Serialize)]
pub struct RejectContestPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

pub async fn reject_contest(id: &str, reason: Option<&str>) -> Result<(), String> {
    let key = contest_key_from_any(id);
    let url = format!("{}/{}/reject", api_url("/api/contests"), key);
    let body = RejectContestPayload {
        reason: reason
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty()),
    };
    let response = authenticated_post(&url)
        .json(&body)
        .map_err(|e| format!("Failed to serialize reject body: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to reject contest: {}", e))?;
    if !(response.ok() || response.status() == 204) {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|_| "Unknown error occurred".to_string())?;
        return Err(error.error);
    }
    Ok(())
}

fn image_mime_allowed(mime: &str) -> bool {
    matches!(
        mime,
        "image/jpeg" | "image/png" | "image/webp" | "image/jpg"
    )
}

/// Read a user-selected image (JPEG/PNG/WebP) for upload.
pub async fn read_contest_image_file(file: web_sys::File) -> Result<(Vec<u8>, String), String> {
    let mime = file.type_();
    if !mime.is_empty() && !image_mime_allowed(&mime) {
        return Err("Use JPEG, PNG, or WebP".to_string());
    }
    let size = file.size();
    if size <= 0.0 {
        return Err("Empty file".to_string());
    }
    if size as usize > MAX_CONTEST_IMAGE_UPLOAD_BYTES {
        return Err(format!(
            "Image must be at most {} MB",
            MAX_CONTEST_IMAGE_UPLOAD_BYTES / (1024 * 1024)
        ));
    }

    let reader = web_sys::FileReader::new().map_err(|_| "FileReader unavailable")?;
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let reject_fail = reject.clone();
        let reject_read = reject.clone();
        let success = wasm_bindgen::closure::Closure::once(move |event: web_sys::ProgressEvent| {
            let target = event.target().unwrap();
            let reader: web_sys::FileReader = target.dyn_into().unwrap();
            match reader.result() {
                Ok(v) => {
                    let _ = resolve.call1(&wasm_bindgen::JsValue::NULL, &v);
                }
                Err(_) => {
                    let _ = reject.call1(
                        &wasm_bindgen::JsValue::NULL,
                        &wasm_bindgen::JsValue::from_str("read failed"),
                    );
                }
            }
        });
        let failure = wasm_bindgen::closure::Closure::once(move |_: web_sys::ProgressEvent| {
            let _ = reject_fail.call1(
                &wasm_bindgen::JsValue::NULL,
                &wasm_bindgen::JsValue::from_str("read failed"),
            );
        });
        reader.set_onloadend(Some(success.as_ref().unchecked_ref()));
        reader.set_onerror(Some(failure.as_ref().unchecked_ref()));
        success.forget();
        failure.forget();
        if reader.read_as_array_buffer(&file).is_err() {
            let _ = reject_read.call1(
                &wasm_bindgen::JsValue::NULL,
                &wasm_bindgen::JsValue::from_str("read failed"),
            );
        }
    });

    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|_| "Failed to read file".to_string())?;
    let buf = js_sys::Uint8Array::new(&value);
    let bytes = buf.to_vec();
    let content_type = if image_mime_allowed(&mime) {
        mime
    } else {
        "application/octet-stream".to_string()
    };
    Ok((bytes, content_type))
}

/// Upload or replace contest thumbnail (server stores resized WebP).
pub async fn upload_contest_image(
    contest_id: &str,
    image_bytes: Vec<u8>,
    content_type: &str,
) -> Result<ContestDto, String> {
    if image_bytes.len() > MAX_CONTEST_IMAGE_UPLOAD_BYTES {
        return Err(format!(
            "Image must be at most {} MB",
            MAX_CONTEST_IMAGE_UPLOAD_BYTES / (1024 * 1024)
        ));
    }
    let key = contest_key_from_any(contest_id);
    let url = format!("{}/{}/image", api_url("/api/contests"), key);
    let body = js_sys::Uint8Array::from(image_bytes.as_slice());
    let response = authenticated_put(&url)
        .header("Content-Type", content_type)
        .body(body)
        .map_err(|e| format!("Failed to build upload request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to upload image: {}", e))?;

    if !response.ok() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        let error = serde_json::from_str::<ErrorResponse>(&body)
            .ok()
            .map(|err| err.error)
            .unwrap_or(body);
        return Err(error);
    }

    response
        .json::<ContestDto>()
        .await
        .map_err(|e| format!("Failed to parse upload response: {}", e))
}

/// Remove contest thumbnail (creator or admin only).
pub async fn delete_contest_image(contest_id: &str) -> Result<(), String> {
    let key = contest_key_from_any(contest_id);
    let url = format!("{}/{}/image", api_url("/api/contests"), key);
    let response = authenticated_delete(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to delete image: {}", e))?;

    if response.status() == 204 {
        return Ok(());
    }

    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string());
    let error = serde_json::from_str::<ErrorResponse>(&body)
        .ok()
        .map(|err| err.error)
        .unwrap_or(body);
    Err(error)
}
