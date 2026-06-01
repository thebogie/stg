# Web frontend and Tauri on production (same backend)

Yes — you can run a **web-based frontend** (browser) and **Tauri** (desktop or mobile) at the same time, both talking to the same production backend.

## How it works

- **One backend** (your Axum API) serves both.
- **Web**: Users open your site in a browser (e.g. `https://smacktalkgaming.com`). The same Yew/WASM app is served as static files; API calls use relative URLs or your domain, so they hit the same backend.
- **Tauri**: Users run the native app (desktop or, with Tauri 2, mobile). The app loads the same frontend (bundled or from your URL) and sends API requests to a **configurable base URL** (`STG_API_URL` → your production API).

So: same API, same app logic; only the “shell” (browser vs native) and how the API base URL is set differ.

## Production setup

### 1. Backend and web on the same domain (recommended)

Typical layout:

- **Frontend**: `https://smacktalkgaming.com` → static files (Trunk build: `dist/` or `_build/frontend-dist/`).
- **API**: Same origin, e.g. `https://smacktalkgaming.com/api` → reverse proxy to your backend (e.g. port 50002).

Your frontend already uses **relative** API URLs in the browser (`api_base_url()` returns `""` in production), so no change is needed: the browser sends requests to the same host and the proxy forwards `/api/*` to the backend.

- **CORS**: Backend already allows `https://smacktalkgaming.com` and `https://www.smacktalkgaming.com` in production (`back/api/src/middleware.rs`). Keep those aligned with the domain you serve the web app from.

### 2. Serving the web frontend

- Build once: from repo root, e.g. `cd front/web && trunk build --release` (or your existing prod build script). Output goes to Trunk’s `dist` (or `_build/frontend-dist` per your `Trunk.toml`).
- On the server, point your web server (e.g. nginx) at that directory:
  - **Document root** = path to that `dist` (or `frontend-dist`) directory.
  - **API**: proxy `/api` (and optionally `/health`, etc.) to the backend, e.g. `http://127.0.0.1:50002`.

Example nginx-style idea (conceptual):

```nginx
# Static frontend
root /path/to/frontend-dist;
try_files $uri $uri/ /index.html;

# Backend
location /api/ {
    proxy_pass http://127.0.0.1:50002;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

So: **one production site** (one domain) serves the web app and the API.

### 3. Tauri pointing at production

- In the Tauri app, the API base URL is set by **`STG_API_URL`** (see `front/tauri/src-tauri/src/commands.rs`). Default is `http://127.0.0.1:50002` for local dev.
- For production builds (desktop or mobile), set `STG_API_URL` to your public API base when building or at runtime, e.g.:
  - `STG_API_URL=https://smacktalkgaming.com` (if API is under same domain), or
  - `STG_API_URL=https://api.smacktalkgaming.com` (if you use a separate API subdomain).

Then:

- **Desktop**: Build the Tauri app (e.g. `cd front/tauri && cargo tauri build`) with that env (or ship a config that sets it). Users install the app; it talks to your production API.
- **Mobile (Tauri 2)**: Same idea: build the app with `STG_API_URL` pointing at your production API; the app on phones uses that URL.

No second “site” is required: the same production API serves web and Tauri.

### 4. CORS when Tauri loads bundled assets

- If the Tauri webview loads your **production URL** (e.g. `https://smacktalkgaming.com`), the request origin is that origin and is already allowed by your CORS config.
- If the Tauri webview loads **bundled** assets (e.g. via a local or custom scheme), the browser may send an origin like `tauri://localhost` or similar. If you see CORS errors from the Tauri app, add that origin in `back/api/src/middleware.rs` (e.g. `allowed_origin("tauri://localhost")` or whatever your Tauri build uses). Many setups instead load `https://smacktalkgaming.com` in the webview so no extra CORS change is needed.

### 5. Auth and sessions

- Web and Tauri both use the same backend (login, cookies/tokens, etc.). Keep using the same auth mechanism (e.g. session cookie for web; for Tauri, same cookie or token in requests). No separate “mobile” backend is required.

## Summary

| Client        | How they reach production      | What you configure |
|---------------|----------------------------------|--------------------|
| Web (browser) | Same domain + reverse proxy      | Serve static build; proxy `/api` to backend; CORS already allows your domain. |
| Tauri desktop| `STG_API_URL` → production URL   | Set `STG_API_URL` for prod builds (or equivalent config). |
| Tauri mobile | Same as desktop                  | Same `STG_API_URL` in the mobile build. |

So: **one production site** (backend + web frontend), **one API**, and Tauri (desktop and phones) points at that API via `STG_API_URL`. No need for a separate “production site” for Tauri.

---

## CI/CD (production pipeline)

The **Production CI/CD** workflow (`.github/workflows/build-and-push.yml`) runs on push to `main` and:

| Job | Output | How to use in production |
|-----|--------|---------------------------|
| **build-backend** | Image pushed to GHCR | On the server: `./deploy_stg.sh <tag>` (pulls image and restarts stack). Tag = `latest` or the short SHA from the run. |
| **build-frontend** | Artifact `frontend-dist` | Download from the run, extract to your web server docroot (or use the deploy-frontend-pages job). |
| **build-tauri** | Artifact `tauri-app` | `.deb` under `_build/target/release/bundle/deb/` (AppImage disabled in CI; use `./scripts/build-tauri.sh prod deb,appimage` locally if needed). |
| **build-tauri-android** | Artifact `tauri-android-apk` | Universal APK when `front/tauri/src-tauri/gen/android/gradlew` is committed; prod API baked via `STG_API_URL`. |
| **deploy-frontend-pages** | (optional) | Run manually: Actions → Production CI/CD → Run workflow, check **Deploy web frontend to GitHub Pages**. Requires GitHub Pages enabled in repo settings (Settings → Pages → Source: GitHub Actions). |

### Deploying backend on the server

- Use the same tag as in the workflow (e.g. `latest` or the commit SHA shown in the job summary):
  - `sudo ./deploy_stg.sh latest`
- Image name: `ghcr.io/<owner>/<repo>/backend:<tag>` (e.g. `ghcr.io/thebogie/stg/backend:latest`). Set `GHCR_IMAGE_BACKEND` in the deploy script if your image name differs.

### Deploying the web frontend

- **Option A**: Download the `frontend-dist` artifact from a successful run, extract it on the server, and point nginx (or your web server) at that directory; proxy `/api` to the backend.
- **Option B**: Enable GitHub Pages and run the workflow with **Deploy web frontend to GitHub Pages** checked. Then either use the Pages URL as your frontend or copy the built files to your own server.

### Tauri production API URL

- **Release builds** default to `https://smacktalkgaming.com` (`front/tauri/src-tauri/src/commands.rs` + `build.rs`). **`STG_API_URL`** still overrides at runtime.
- **Local builds:** `./scripts/build-tauri.sh` (desktop) and `./scripts/build-tauri-android.sh` (APK) set prod API unless you pass `dev` or export another URL. See `env.tauri.prod.template`.

### Commit Android scaffold for CI

After `cargo tauri android init`, commit `front/tauri/src-tauri/gen/android/` **except** build caches (`build/`, `.gradle/` are gitignored there). Without `gradlew` in git, the `build-tauri-android` job is skipped.
