//! Tauri bridge: detect Tauri and invoke Rust commands from the Yew (WASM) frontend.
//! When not running in Tauri, these are no-ops or return None.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// App config returned by the Tauri command `get_app_config`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub api_base_url: String,
    pub is_tauri: bool,
}

/// Returns true when the app is running inside the Tauri desktop shell.
pub fn is_tauri() -> bool {
    let Some(win) = web_sys::window() else {
        return false;
    };
    // Tauri v2 with withGlobalTauri: true exposes window.__TAURI__
    let tauri = js_sys::Reflect::get(&win, &JsValue::from_str("__TAURI__"));
    let Ok(tauri) = tauri else {
        return false;
    };
    !tauri.is_undefined() && !tauri.is_null()
}

/// Invoke a Tauri command. Returns the Promise so the caller can .await via JsFuture.
/// When not in Tauri, returns None (caller should fall back to browser behavior).
pub fn invoke_tauri(cmd: &str, args: JsValue) -> Option<js_sys::Promise> {
    let win = web_sys::window()?.dyn_into::<JsValue>().ok()?;
    // Tauri v2: window.__TAURI__.core.invoke(cmd, args)
    let tauri = js_sys::Reflect::get(&win, &JsValue::from_str("__TAURI__")).ok()?;
    if tauri.is_undefined() || tauri.is_null() {
        return None;
    }
    let core = js_sys::Reflect::get(&tauri, &JsValue::from_str("core")).ok()?;
    if core.is_undefined() || core.is_null() {
        return None;
    }
    let invoke_fn = js_sys::Reflect::get(&core, &JsValue::from_str("invoke")).ok()?;
    if invoke_fn.is_undefined() || invoke_fn.is_null() {
        return None;
    }
    let invoke_fn = invoke_fn.dyn_ref::<js_sys::Function>()?;
    let promise = invoke_fn
        .call2(&core, &JsValue::from_str(cmd), &args)
        .ok()?;
    let promise = promise.dyn_into::<js_sys::Promise>().ok()?;
    Some(promise)
}

/// Load app config from Tauri. When not in Tauri, returns None (caller uses browser origin).
pub async fn get_app_config_from_tauri() -> Option<AppConfig> {
    let promise = invoke_tauri("get_app_config", JsValue::NULL)?;
    let result = wasm_bindgen_futures::JsFuture::from(promise).await.ok()?;
    let config: AppConfig = serde_wasm_bindgen::from_value(result).ok()?;
    Some(config)
}
