# Agent handbook (STG / stg_rd)

Guidance for **humans and AI agents** working in parallel on this repo. For layout details, **`docs/PROJECT_STRUCTURE.md`** is the source of truth.

## Parallel work: git worktrees

Give each agent (or long-running task) its **own directory and branch** so nobody shares a dirty working tree.

```bash
# From your main clone
git worktree add ../stg-agent-back -b agent/back-short-description
git worktree add ../stg-agent-web -b agent/web-short-description
```

Merge through normal PRs. Tear down when done: `git worktree remove ../stg-agent-back` (after merging or abandoning the branch).

## Ownership by area

| Area | Paths | Notes |
|------|--------|--------|
| Backend API | `back/api/` | Cargo package name **`backend`** (not `back` or `api`). |
| Compose / stack | `deploy/` | SurrealDB, Redis, backend container; used by `./scripts/start-back.sh` and CI. |
| Web (Yew / WASM) | `front/web/` | Trunk + Yew; `npm` in this dir; run `./scripts/start-front.sh` from repo root. |
| Tauri shell | `front/tauri/` | Embeds the same Yew app; `cargo tauri` from `front/tauri`. |
| Shared types | `shared/` | Used by backend, frontend, and tests—coordinate contract changes. |
| Integration tests | `testing/` | Needs stack up for integration runs (see `docs/testing/HOW_TO_RUN_TESTS.md`). |
| Env & scripts | `config/`, `scripts/`, `data/` | Env: `./config/setup-env.sh dev|prod`. Local Docker state: gitignored **`data/`** (see `scripts/load-env.sh` for `VOLUME_PATH`). |

Avoid assigning two agents to the same hotspot in one iteration (see below).

## Domain / business rules

- **Contest outcome `score` is game-specific.** Each title can define scoring differently (VP, currency, win points, etc.). **Do not assume scores are comparable across games** in analytics, leaderboards, or ratings unless you add an explicit, documented normalization (e.g. per-game scaling or categories).

## Hot merge points (serialize or single owner)

- **Root `Cargo.toml` and `Cargo.lock`** — workspace definition and lockfile; one agent per PR slice.
- **`shared/`** — API/Web contract; prefer one agent or merge backend+shared before frontend consumes types.
- **Workspace `version` / `[workspace.package]`** — coordinated with releases.

## Quick commands

- Full **prod-image** gate (same as push-ready): `./scripts/test-prod-gate.sh`
- Quick **prod-like** smoke: `./scripts/test-prod-like-smoke.sh` or `./ci-local.sh smoke prod`
- Full local CI (compose stages): `./ci-local.sh all`
- **Dev + breakpoints:** `./scripts/dev-debug.sh` then `just backend-watch`
- Backend stack (all in Docker): `./scripts/start-back.sh` · stop: `./scripts/stop-back.sh`
- Frontend: `./scripts/start-front.sh` · Tauri: `./scripts/start-tauri.sh`
- Deps only (backend on host): `./scripts/start-deps.sh` then e.g. `just backend-watch`
- **Server:** install GHCR images: `./scripts/install-from-ci.sh <tag>` (see `deploy/README.md`)

## Cursor rules

Path-scoped rules live in **`.cursor/rules/`** (`*.mdc`). They reinforce the same boundaries; keep changes aligned with this file when you add new crates or top-level dirs.

## Docs index

- `docs/PROJECT_STRUCTURE.md` — tree and commands
- `docs/WORKFLOW.txt` — CI, backend, frontend, production
- `docs/setup/DEVELOPMENT_SETUP.md` — environment
- `docs/testing/HOW_TO_RUN_TESTS.md` — tests
