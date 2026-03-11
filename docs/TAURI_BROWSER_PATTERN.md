# Tauri + Browser: One Frontend, Two Runtimes

We use the **single codebase, runtime detection, conditional config** pattern so the same Yew/Trunk frontend runs in both the **browser** and the **Tauri** desktop shell.

## Pattern (what’s standard)

1. **One frontend** – Same SPA (HTML/CSS/JS/WASM) for web and desktop. No separate “Tauri build” of the UI.
2. **Runtime detection** – At runtime, check `window.__TAURI__` (Tauri v2 with `withGlobalTauri: true`) to know if we’re in the desktop shell.
3. **Conditional API** – Use Tauri-only APIs (e.g. `invoke`) only when `window.__TAURI__` is present; in the browser use normal `window`/`fetch`/origin.
4. **Same backend** – Both environments talk to the same API (e.g. `back/api`). Only *how* we get the API base URL differs (see below).

References: [Tauri frontend config](https://v2.tauri.app/start/frontend/), [detecting Tauri](https://github.com/tauri-apps/tauri/issues/933), [single codebase](https://andamp.io/blog/tauri-v2-one-codebase-4-all).

## How we implement it

| Concern | Browser | Tauri |
|--------|---------|--------|
| **Detection** | `window.__TAURI__` absent | `window.__TAURI__` present |
| **Config** | From `window.location.origin` (localhost → `http://127.0.0.1:50002`, else `""`) | From Rust: `invoke('get_app_config')` (env `STG_API_URL` or default `http://127.0.0.1:50002`) |
| **When config is set** | Synchronously in `ConfigLoader` before rendering app | After async `get_app_config_from_tauri()` in `ConfigLoader` (short “Loading…” then app) |
| **API requests** | Same `gloo_net` / `api_url(path)` in both | Same; in Tauri dev we use relative URLs when origin is localhost so Trunk’s proxy forwards and the webview doesn’t block cross-origin |

So:

- **Single entry point for “where is the API?”** – `config::api_base_url()` (used by `api_url()`). Rest of the app always uses `api_url(path)` and doesn’t care if we’re in Tauri or browser.
- **Single place for “are we in Tauri?”** – `config::is_tauri()` (backed by `tauri::is_tauri()`). Use this only when you need Tauri-specific behavior (e.g. calling `invoke`).

## Files

| File | Role |
|------|------|
| `front/web/src/tauri.rs` | `is_tauri()`, `invoke_tauri()`, `get_app_config_from_tauri()` – only used when we need to talk to the Tauri shell. |
| `front/web/src/config.rs` | `api_base_url()`, `is_tauri()`, `set_app_config` / `get_app_config` – global config and runtime detection. |
| `front/web/src/components/config_loader.rs` | On load: if Tauri → invoke `get_app_config` then set config; else set config from origin. Renders app only when config is ready (in Tauri after invoke, in browser immediately). |
| `front/web/src/api.rs` | `api_url(path)` uses `config::api_base_url()` so all HTTP stays environment-agnostic. |
| `front/tauri/src-tauri/src/commands.rs` | Tauri command `get_app_config` returns `{ apiBaseUrl, isTauri }` (Rust reads `STG_API_URL` or default). |

## Adding Tauri-only behavior

- **New Rust command:** Add in `front/tauri/src-tauri/src/commands.rs`, register in `lib.rs`, call from the frontend with `tauri::invoke_tauri("command_name", args)`.
- **Guard:** Use `if config::is_tauri() { ... } else { ... }` so browser code never touches Tauri APIs.

## Summary

- **Pattern:** One frontend, runtime check `window.__TAURI__`, conditional config and optional Tauri APIs; same `api_url()` and fetch for both.
- **We do not:** Maintain two UIs or two build pipelines; we have one Trunk build used by both browser and Tauri.
