# Sell a Game — Guardrails

Human-assisted listing pipeline: player defaults → photos → listing → Playwright fills BGG GeekMarket.

## BGG credentials

- STG **never stores BGG passwords** in the database.
- Username may be saved in `sell_preferences` for convenience.
- Password is sent over HTTPS only when the player clicks **Fill BGG marketplace form**, held briefly in Redis for the Playwright worker, then discarded.
- Operators must comply with [BGG Terms of Use](https://boardgamegeek.com/wiki/page/Terms_of_Use).

## Automation

Two modes via `PLAYWRIGHT_MODE`:

| Mode | When | Behavior |
|------|------|----------|
| `local` | Hybrid dev (backend on host) | Backend spawns `tools/bgg-marketplace/fill-listing.mjs` via Node on the host |
| `queue` | Production Docker stack | Backend enqueues Redis job; `playwright-worker` container runs the script |

Production (`deploy/docker-compose.yml`):

- **`stg-playwright-worker`** — Node + Chromium, consumes `playwright:queue`
- Shared volume `${VOLUME_PATH}/backend_data` — sell listing photos (`/app/data/sell-images`)
- Shared volume `${VOLUME_PATH}/playwright_jobs` — per-job logs under `/jobs/{job_id}/`
- API returns `job_id`; UI polls `GET .../automate/{job_id}/status` until `completed` or `failed`

Build worker image: `./scripts/build-playwright-worker-image.sh` (local dev only; production uses GHCR via `./deploy/deploy_stg.sh <tag>`).

Default: headless fill, **stops before final BGG submit** (`BGG_AUTO_SUBMIT=0`).

## Image retention

| Setting | Default |
|---------|---------|
| `SELL_IMAGE_TTL_HOURS` | 24 |
| `SELL_IMAGE_MAX_COUNT` | 8 |
| `SELL_IMAGE_MAX_BYTES` | 8 MiB |

Photos are ephemeral; deleted on cancel, submit, or expiry.

## Player defaults (`/api/sell/preferences`)

Stored per player: currency, default condition, payment methods, item location, ship-to, seller notes, optional BGG username.

## Workflow checkpoints

1. **preferences** — sell defaults saved
2. **photos** — at least one box photo uploaded
3. **listing** — game, price, notes confirmed
4. **automate** — Playwright fills BGG form (password at this step only)

## Environment

| Variable | Purpose |
|----------|---------|
| `PLAYWRIGHT_MODE` | `local` (dev) or `queue` (prod Docker) |
| `PLAYWRIGHT_JOB_TTL_SECONDS` | Redis job payload TTL (default 900) |
| `PLAYWRIGHT_STATUS_TTL_SECONDS` | Job status TTL for polling (default 3600) |
| `NODE_BIN` | Path to node for `local` mode (default `node`) |
| `BGG_PLAYWRIGHT_SCRIPT` | Override script path (`local` mode) |
| `BGG_HEADLESS` | `1` headless (default), `0` headed |
| `BGG_AUTO_SUBMIT` | `1` to click BGG submit |
