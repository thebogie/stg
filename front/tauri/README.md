# STG Tauri app (desktop / mobile)

Tauri shell that **embeds the same Yew frontend** as the website (`front/web`). One codebase for web and native.

- **Dev**: Tauri starts the Trunk dev server and loads `http://localhost:50003`.
- **Build**: Tauri runs `trunk build` in `front/web`, then bundles `_build/frontend-dist` into the app.

## Prerequisites

- Rust, Trunk, and the same setup as `front/web`.
- [Tauri CLI](https://v2.tauri.app/start/install/): `cargo install tauri-cli --version "^2.0.0"`
- **Linux desktop**: webkit2gtk and other [Tauri system deps](https://v2.tauri.app/start/install/#linux).
- **Android**: JDK 17, Android SDK, NDK — [mobile prerequisites](https://v2.tauri.app/start/prerequisites/#android).

## Quick commands (from repo root)

| Goal | Command |
|------|---------|
| Desktop dev | `./scripts/start-tauri.sh` (backend should be up) |
| Desktop `.deb` (prod API) | `./scripts/build-tauri.sh` |
| Desktop `.deb` (local API) | `./scripts/build-tauri.sh dev` |
| Android APK (prod API) | `./scripts/build-tauri-android.sh` |

Installers use workspace target dir: **`_build/target/release/bundle/`** (see `.cargo/config.toml`).

## Run (development)

From this directory (`front/tauri`):

```bash
cargo tauri dev
```

Or from repo root: `./scripts/start-tauri.sh`

Ensure the backend is running (e.g. `./scripts/start-back.sh`) so the app can call the API.

## Build (production)

**Desktop** — prefer the script (sets `STG_API_URL`, Tailwind, bundles):

```bash
./scripts/build-tauri.sh
```

Manual:

```bash
cd front/tauri
STG_API_URL=https://smacktalkgaming.com cargo tauri build --bundles deb
```

Output: `../../_build/target/release/bundle/deb/*.deb`

Optional local bundles (AppImage can fail on flaky networks):

```bash
./scripts/build-tauri.sh prod deb,rpm,appimage
```

**Android** — one-time init, then build:

```bash
cd front/tauri
cargo tauri android init   # once per machine / after clone if gen/android missing
cd ../..
./scripts/build-tauri-android.sh
```

APK under: `front/tauri/src-tauri/gen/android/app/build/outputs/apk/`

Commit the **`gen/android`** scaffold (not `build/` or `.gradle/`) so GitHub Actions can build APKs.

## Structure

| Path | Role |
|------|------|
| **front/web** | Yew (Trunk) app — shared by browser and Tauri |
| **front/tauri** | Tauri project root (run `cargo tauri` here) |
| **front/tauri/src-tauri** | Rust shell: config, window, plugins |
| **back/api** | Same API for web and Tauri |

## Configuration

- **tauri.conf.json**: `bundle.targets` is `["deb"]` by default (avoids flaky AppImage downloads in CI).
- **API URL**: `get_app_config` in Rust — debug default `http://127.0.0.1:50002`, release default `https://smacktalkgaming.com`; override with **`STG_API_URL`**.
- See also: [`deploy/WEB_AND_TAURI.md`](../../deploy/WEB_AND_TAURI.md), [`deploy/env.tauri.prod.template`](../../deploy/env.tauri.prod.template).

## Mobile (iOS / Android)

Install [Tauri mobile deps](https://v2.tauri.app/start/mobile/), then from `front/tauri`:

- `cargo tauri android dev` / `cargo tauri android build`
- `cargo tauri ios dev` / `cargo tauri ios build` (macOS only)

**Phone not on same machine as PC:** build APK with `./scripts/build-tauri-android.sh`, copy to Google Drive, install on device.

**CI:** Production workflow builds `.deb` always; Android APK when `gen/android/gradlew` is in the repo (artifact `tauri-android-apk`).
