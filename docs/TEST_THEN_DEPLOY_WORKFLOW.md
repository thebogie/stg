# Test-then-deploy workflow

Run tests locally, then deploy via GHCR. No local production image build or export in this repo.

## 1. Test locally

```bash
./ci-local.sh all
```

This runs build, unit tests, starts the stack (SurrealDB, Redis, backend), runs integration tests, runs E2E smoke, then brings the stack down. Uses `config/.env.prod` and `deploy/docker-compose.yml`. Do this before pushing to `main`.

## 2. Deploy to production

1. Push to `main`. GitHub Actions builds the backend image and pushes it to GHCR.
2. On the production server: pull the image and run your stack. See **[GHCR_SETUP.md](GHCR_SETUP.md)** for one-time setup and the exact commands.

Containers are built in CI from the same code you tested with `ci-local.sh all`. Data (SurrealDB, Redis volumes) stays on the server; only the backend container is replaced when you pull a new image.

## Production network (reference)

If you use Traefik in front of the app:

- Route `/` (and non-API routes) → frontend (e.g. port 50003).
- Route `/api/*` → backend (e.g. port 50002).

Backend talks to SurrealDB and Redis via Docker service names (`surrealdb`, `redis`) inside the compose network. Port mappings (e.g. SurrealDB 50001, backend 50002, frontend 50003, Redis 6379) are set in your env and compose.

## Summary

| Step   | Command / action |
|--------|-------------------|
| Test   | `./ci-local.sh all` |
| Deploy | Push to main → Actions push image to GHCR → on server: pull and `docker compose up` (see GHCR_SETUP.md) |

For day-to-day dev workflow, see [DAILY_WORKFLOW.md](DAILY_WORKFLOW.md).
