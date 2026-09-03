# Documentation index

Start here for project docs. **Daily dev:** [DAILY_WORKFLOW.md](DAILY_WORKFLOW.md). Repo layout: [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md). CI/script index: [WORKFLOW.txt](WORKFLOW.txt). The project follows **Rust, Yew, Tauri, SurrealDB, and Redis**; docs in **[archive/](archive/README.md)** (especially `archive/outdated-stack/`) are obsolete and for reference only.

## Quick links

### Getting started
- **[Daily workflow](DAILY_WORKFLOW.md)** – **Start here:** local dev, prod snapshot import, test, deploy
- **[Project structure](PROJECT_STRUCTURE.md)** – Repo layout, back/api, front/web, config, scripts
- **[Development setup](setup/DEVELOPMENT_SETUP.md)** – One-time env setup and troubleshooting
- **[Workflow](WORKFLOW.txt)** – CI and script entrypoints (one-page reference)
- **[Setup project structure](setup/PROJECT_STRUCTURE.md)** – Points to main PROJECT_STRUCTURE
- **[Migration guide](setup/MIGRATION_GUIDE.md)** – Migrating from old structure
- **[Documentation organization](DOCUMENTATION_ORGANIZATION.md)** – How docs are organized

### Testing
- **[How to run tests](testing/HOW_TO_RUN_TESTS.md)** – Run all tests (just / ci-local)
- **[E2E testing guide](testing/E2E_TESTING_GUIDE.md)** – Playwright E2E
- **[Testing setup](testing/TESTING_SETUP.md)** – Test environment
- **[Testing architecture](testing/TESTING_ARCHITECTURE.md)** – Test design
- Other testing docs live in `docs/testing/` (tiers, coverage, reporting, etc.)

### API
- **[Authentication API](api/AUTHENTICATION_API.md)** – Auth endpoints

### Architecture and standards
- **[SurrealDB ID conventions](SURREALDB_ID_CONVENTIONS.md)** – Record IDs (Thing vs string); follow for all SurrealDB access
- **[Tauri + browser pattern](TAURI_BROWSER_PATTERN.md)** – One Yew frontend, browser and Tauri runtimes
- **[Client analytics](architecture/CLIENT_ANALYTICS_ARCHITECTURE.md)** – Analytics design
- **[Glicko2 ratings](GLICKO2_RATINGS_IMPLEMENTATION.md)** – Rating system
- **[Backend scheduler](BACKEND_SCHEDULER_IMPLEMENTATION.md)** – Scheduler
- **[Admin authorization](ADMIN_AUTHORIZATION_SYSTEM.md)** – Admin system

### Deployment and workflows
- **[Daily workflow](DAILY_WORKFLOW.md)** – Local dev, prod snapshot, test, deploy
- **[GHCR setup](GHCR_SETUP.md)** – **Primary deployment**: build on push, pull from GHCR on production
- **[Test-then-deploy workflow](TEST_THEN_DEPLOY_WORKFLOW.md)** – Test before deploy (references GHCR)
- **[CI/CD](CI_CD.md)** – Pipeline overview (SurrealDB + Redis, ci-local.sh)
- **[Deploy directory](../deploy/README.md)** – Docker Compose and usage

## Doc layout

| Area       | Path                 |
|-----------|----------------------|
| Setup     | `docs/setup/`        |
| Testing   | `docs/testing/`      |
| API       | `docs/api/`          |
| Architecture | `docs/architecture/` |
| Root docs | `docs/*.md`, `docs/WORKFLOW.txt` |
| Archive   | `docs/archive/`      |

When adding docs, put them in the right category and add a link above or in the matching section.
