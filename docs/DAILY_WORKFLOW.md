# Daily development workflow

Typical day-to-day flow: code locally, run tests, push to main; production pulls the backend image from GHCR.

## 1. Develop locally

- **Terminal 1:** `./scripts/start-back.sh` (or `./scripts/start-deps.sh` + `just backend-watch` for hybrid).
- **Terminal 2:** `./scripts/start-front.sh` or `./scripts/start-tauri.sh`.

Use `config/.env.dev`. See [setup/DEVELOPMENT_SETUP.md](setup/DEVELOPMENT_SETUP.md) and [WORKFLOW.txt](WORKFLOW.txt).

## 2. Run tests before pushing

```bash
./ci-local.sh all
```

Runs build, unit tests, integration tests (with stack up), and E2E smoke. Uses `config/.env.prod` and `deploy/docker-compose.yml`.

## 3. Deploy to production

1. Push to `main`. GitHub Actions builds the backend image and pushes to GHCR.
2. On the production server: pull the image and start the stack. See **[GHCR_SETUP.md](GHCR_SETUP.md)** for setup and commands.

There are no scripts in this repo for building production images locally, exporting them, or deploying from tarballs; the current path is **push to main → GHCR → pull on production**.

## Summary

| Step       | What you do |
|------------|-------------|
| Develop    | `start-back.sh` + `start-front.sh` (or hybrid with `start-deps.sh` and `backend-watch`) |
| Test       | `./ci-local.sh all` |
| Deploy     | Push to main; on server: pull from GHCR and run compose (see GHCR_SETUP.md) |
