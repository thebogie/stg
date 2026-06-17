# STG Playwright worker

Redis-backed browser automation service for production Docker.

Runs from the **unified** `stg-playwright` image (`deploy/Dockerfile.playwright`). The same image includes `@playwright/test` for CI E2E; production starts only the worker daemon (default `CMD`).

## Job types

| `job_type` | Script |
|------------|--------|
| `bgg.geekmarket.fill` | `tools/bgg-marketplace/fill-listing.mjs` |

Add new rows by extending `worker.mjs` `dispatch()` and enqueueing from the Rust API.

## Redis keys

- `playwright:queue` — job IDs (LPUSH / BRPOP)
- `playwright:job:{id}` — full job JSON incl. ephemeral credentials (TTL)
- `playwright:status:{id}` — pollable status for the API

## Local run (with Redis)

```bash
cd tools/playwright-worker && npm install
REDIS_URL=redis://127.0.0.1:6379/ node worker.mjs
```

## Docker

```bash
./scripts/build-playwright-image.sh
```

Started automatically by `deploy/docker-compose.yml` as `playwright-worker`.

CI E2E: `./scripts/run-playwright-e2e-docker.sh` (same image, `playwright test` entrypoint).
