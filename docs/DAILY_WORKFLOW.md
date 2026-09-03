# Daily development workflow

**Start here** for day-to-day local dev. One-time env setup and troubleshooting: [setup/DEVELOPMENT_SETUP.md](setup/DEVELOPMENT_SETUP.md). Script index: [WORKFLOW.txt](WORKFLOW.txt), [scripts/README.md](../scripts/README.md).

Typical flow: code locally → `./ci-local.sh all` → push to `main` → production pulls from GHCR ([GHCR_SETUP.md](GHCR_SETUP.md)).

---

## One-time setup

```bash
./config/setup-env.sh dev    # creates config/.env.dev
just setup                   # optional: cargo-watch, nextest, Playwright (hybrid dev)
```

Add to `config/.env.dev` if you use the prod snapshot (see below):

```bash
SURREAL_SEED_DIR=/home/thebogie/work/stg-data/prod-db
```

---

## Prod snapshot → local dev

Production Surreal exports live at **`/home/thebogie/work/stg-data/prod-db`** (`*.surql.gz`, e.g. `stg_rd.surql.gz`, `system.surql.gz`).

Dev scripts read them via **`SURREAL_SEED_DIR`**. Use **`SURREAL_SEED_FORCE=1`** whenever you want a fresh import (required on first seed and when refreshing from prod — after SurrealDB starts, `data/dev/surrealdb_data` is never empty).

### First run or refresh from prod

**Full stack (backend in Docker):**

```bash
# Terminal 1
SURREAL_SEED_DIR=/home/thebogie/work/stg-data/prod-db SURREAL_SEED_FORCE=1 ./scripts/start-back.sh

# Terminal 2
./scripts/start-front.sh          # browser
# or: ./scripts/start-tauri.sh    # desktop
```

**Hybrid (breakpoints / fast backend iteration):**

```bash
# Terminal 1
SURREAL_SEED_DIR=/home/thebogie/work/stg-data/prod-db SURREAL_SEED_FORCE=1 ./scripts/start-deps.sh
# same as: ... ./scripts/dev-debug.sh

# Terminal 2
just backend-watch

# Terminal 3
./scripts/start-front.sh
```

Hybrid also applies `tools/arango-to-surreal/surreal-functions.surql` after import. Full Docker (`start-back.sh`) does not — run `./scripts/apply-surreal-functions.sh` if needed.

(Omit `SURREAL_SEED_DIR=...` if you added it to `.env.dev`.)

### Normal day (keep existing local DB)

```bash
# Full stack
./scripts/start-back.sh              # or --no-build for faster restart
./scripts/start-front.sh

# Hybrid
./scripts/start-deps.sh
just backend-watch
./scripts/start-front.sh
```

Stop full stack: `./scripts/stop-back.sh`.

---

## Develop without prod snapshot

Empty or existing `data/dev` — no seed import.

| Mode | Terminal 1 | Terminal 2 | Terminal 3 |
|------|------------|------------|------------|
| **Full Docker** | `./scripts/start-back.sh` | `./scripts/start-front.sh` | — |
| **Hybrid** | `./scripts/start-deps.sh` | `just backend-watch` | `./scripts/start-front.sh` |

**Switch full stack → hybrid** (keep DB, no re-import):

```bash
docker stop stg-backend
just backend-watch
./scripts/start-front.sh
```

Legacy paths still work: Arango zip at `~/work/_backups/smacktalk.zip` or Surreal export at `_build/smacktalk.surql` (see script headers in `start-back.sh`).

---

## Verify

```bash
curl -s http://127.0.0.1:50002/health
# Surrealist: http://127.0.0.1:50001  ns/db: stg_rd / stg_rd
./scripts/verify-contest-scores.sh   # optional, after prod seed
```

### Optional: Grafana (logs)

After the main stack is up (`start-back.sh` or `start-deps.sh`):

```bash
./scripts/start-observability.sh
```

Open **http://localhost:50004** in the VM (user `admin`, password `GRAFANA_ADMIN_PASSWORD` in `config/.env.dev`, default `changeme`). Loki and Prometheus stay on the Docker network only — you browse logs through Grafana, not by exposing those ports.

**Hyper-V / Windows host:** forward the same way as `50001`–`50003` (Admin PowerShell; VM IP e.g. `192.168.200.10`):

```powershell
netsh interface portproxy add v4tov4 listenaddress=0.0.0.0 listenport=50004 connectaddress=192.168.200.10 connectport=50004
```

Then use **http://localhost:50004** on Windows. List existing rules: `netsh interface portproxy show all`.

- **Full Docker** (`start-back.sh`): backend logs appear in Grafana (Promtail scrapes Docker containers).
- **Hybrid** (`just backend-watch`): use the terminal for backend logs; Grafana still shows SurrealDB, Redis, and other containers.

LogQL examples and incident runbook: [observability/LOGGING.md](observability/LOGGING.md).

---

## Run tests before pushing

```bash
./ci-local.sh all
```

Build, unit, integration (stack up), E2E smoke. Uses `config/.env.prod` and `deploy/docker-compose.yml`. Integration tests can seed from the same prod snapshot via `PROD_DB_SNAPSHOT_DIR` (see `scripts/ci.sh`).

---

## Deploy to production

1. Push to `main` — GitHub Actions builds and pushes the backend image to GHCR.
2. On the server: pull and start. See **[GHCR_SETUP.md](GHCR_SETUP.md)**.

No local tarball deploy path; flow is **push → GHCR → pull on production**.

---

## Quick reference

| Goal | Command |
|------|---------|
| Prod data, full stack | `SURREAL_SEED_FORCE=1 ./scripts/start-back.sh` (+ `SURREAL_SEED_DIR` if not in `.env.dev`) |
| Prod data, hybrid | `SURREAL_SEED_FORCE=1 ./scripts/start-deps.sh` + `just backend-watch` |
| Normal dev | `start-back.sh` or `start-deps.sh` + front (no `FORCE`) |
| Grafana logs | `./scripts/start-observability.sh` → http://localhost:50004 |
| Test | `./ci-local.sh all` |
| Deploy | Push `main`; server pull from GHCR |
