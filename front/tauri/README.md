# STG Tauri app (desktop / mobile)

Tauri shell that **embeds the same Yew frontend** as the website (`front/web`). One codebase for web and native.

- **Dev**: Tauri starts the Trunk dev server and loads `http://localhost:50003`.
- **Build**: Tauri runs `trunk build` in `front/web`, then bundles `_build/frontend-dist` into the app.

## Prerequisites

- Rust, Trunk, and the same setup as `front/web`.
- [Tauri CLI](https://v2.tauri.app/start/install/): `cargo install tauri-cli`
- For Linux: webkit2gtk and other [Tauri system deps](https://v2.tauri.app/start/install/#linux).

## Run (development)

From this directory (`front/tauri`):

```bash
cargo tauri dev
```

This will:

1. Start `trunk serve` in `front/web` (port 50003).
2. Open the Tauri window loading that URL.

Ensure the backend is running (e.g. `back/api` on port 50002) so the app can call the API.

## Build (production)

From `front/tauri`:

```bash
cargo tauri build
```

This runs `trunk build` in `front/web`, then builds the Tauri binary and bundles the Yew output. The binary is in `front/tauri/src-tauri/target/release/` (or `target/debug/` for unoptimized).

## Structure

| Path | Role |
|------|------|
| **front/web** | Yew (Trunk) app — shared by browser and Tauri |
| **front/tauri** | Tauri project root (run `cargo tauri` here) |
| **front/tauri/src-tauri** | Rust shell: config, window, plugins |
| **back/api** | Same API for web and Tauri |

## Configuration

- **tauri.conf.json** (in `src-tauri/`): `beforeDevCommand` / `beforeBuildCommand` run Trunk in `../web`; `devUrl` is `http://localhost:50003`; `frontendDist` is `../../../_build/frontend-dist`.
- **Trunk** (`front/web/Trunk.toml`): `ws_protocol = "ws"` for Tauri dev; build output goes to `_build/frontend-dist`.
- **API URL**: The frontend gets the backend base URL from the Tauri command `get_app_config` (invoked once on load). Set env **`STG_API_URL`** to override the default `http://127.0.0.1:50002` (e.g. for production or a different host).

## Mobile (iOS / Android)

The same Tauri project can target mobile. Install [Tauri mobile deps](https://v2.tauri.app/start/mobile/), then from `front/tauri`:

- `cargo tauri android dev` / `cargo tauri android build`
- `cargo tauri ios dev` / `cargo tauri ios build`

Use the same backend URL (or a deployed API) in the app config if needed.
