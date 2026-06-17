use crate::api::api_url;
use web_sys::window;

/// Simple consent-aware event tracker. Stores a minimal consent flag in localStorage.
pub fn track_event(name: &str, props: serde_json::Value) {
    if !has_consent() {
        return;
    }

    web_sys::console::log_1(&format!("analytics_event: {} {}", name, props).into());

    let name = name.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let body = serde_json::json!({ "name": name, "props": props });
        if let Ok(req) = gloo_net::http::Request::post(&api_url("/api/client/events"))
            .header("Content-Type", "application/json")
            .body(body.to_string())
        {
            let _ = req.send().await;
        }
    });
}

pub fn grant_consent() {
    if let Some(win) = window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let _ = storage.set_item("analytics_consent", "true");
        }
    }
}

pub fn has_consent() -> bool {
    if let Some(win) = window() {
        if let Ok(Some(storage)) = win.local_storage() {
            if let Ok(val) = storage.get_item("analytics_consent") {
                return val.unwrap_or_default() == "true";
            }
        }
    }
    false
}

pub fn track_login_click(location: &str, authenticated: bool) {
    track_event(
        "login_click",
        serde_json::json!({
            "location": location,
            "authenticated": authenticated,
        }),
    );
}

pub fn track_cta_create_contest_click(location: &str) {
    track_event(
        "cta_create_contest_click",
        serde_json::json!({
            "location": location,
        }),
    );
}
