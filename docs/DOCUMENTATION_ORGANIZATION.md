# Documentation organization

How documentation is organized and where to add or find things. The project follows **Rust, Yew, Tauri, SurrealDB, and Redis**; docs in `archive/` (especially `archive/outdated-stack/`) do not reflect current standards and are reference only.

## Structure

```
docs/
├── README.md                    # Documentation index (start here)
├── DAILY_WORKFLOW.md            # Primary dev cheat sheet (prod snapshot, start commands)
├── WORKFLOW.txt                 # Short workflow: CI, backend, frontend, production
├── PROJECT_STRUCTURE.md         # Single source of truth for repo layout
├── SURREALDB_ID_CONVENTIONS.md  # Record IDs (Thing vs string) — follow this
├── setup/                       # Setup and development
│   ├── DEVELOPMENT_SETUP.md
│   ├── PROJECT_STRUCTURE.md    # → points to ../PROJECT_STRUCTURE.md
│   ├── MIGRATION_GUIDE.md
│   └── SURREALDB_UI.md
├── testing/                     # Testing docs
│   ├── HOW_TO_RUN_TESTS.md
│   ├── E2E_TESTING_GUIDE.md
│   ├── TESTING_ARCHITECTURE.md
│   └── ...
├── api/                         # API documentation
│   └── AUTHENTICATION_API.md
├── architecture/                # Architecture
│   ├── CLIENT_ANALYTICS_ARCHITECTURE.md
│   └── CLIENT_ANALYTICS_README.md
└── archive/                     # Old or superseded docs (reference only)
    ├── outdated-stack/           # ArangoDB / old paths / old workflow — do not follow
    └── ...
```

## Root-level docs (project root)

- **README.md** – Main project overview and getting started. Points to `docs/` for setup and workflow.

## Deployment

- **Primary:** `docs/GHCR_SETUP.md` – Build on push to main, pull from GHCR on production.
- Workflow docs (e.g. `DAILY_WORKFLOW.md`, `TEST_THEN_DEPLOY_WORKFLOW.md`) reference this for production deploy.

## Categories

| Category    | Path              | Use for |
|------------|-------------------|--------|
| Setup      | `docs/setup/`     | Environment, project structure, migration |
| Testing    | `docs/testing/`   | How to run tests, E2E, coverage, architecture |
| API        | `docs/api/`       | Endpoint and auth documentation |
| Architecture | `docs/architecture/` | System design, analytics, etc. |
| Root `docs/` | (no subdir)      | Workflows, CI/CD, feature-specific docs |

## Finding docs (canonical — follow these)

1. **Start:** `docs/README.md` – Index and quick links.
2. **Daily dev:** `docs/DAILY_WORKFLOW.md` – Local start commands, prod snapshot import, test, deploy.
3. **Structure:** `docs/PROJECT_STRUCTURE.md` – Repo layout and run commands (back/api, front/web, SurrealDB, Redis).
4. **Workflow / CI:** `docs/WORKFLOW.txt` – Script entrypoints in one page.
5. **Setup (one-time):** `docs/setup/DEVELOPMENT_SETUP.md` – Env files, troubleshooting.
6. **Testing:** `docs/testing/HOW_TO_RUN_TESTS.md`.
7. **Deployment:** `docs/GHCR_SETUP.md`.
8. **SurrealDB IDs:** `docs/SURREALDB_ID_CONVENTIONS.md` – Record id vs string, type::record (v3), INSIDE bindings.
9. **Tauri + browser:** `docs/TAURI_BROWSER_PATTERN.md` – One frontend, two runtimes.

## Adding or moving docs

1. Put new docs in the right category (setup / testing / api / architecture or root `docs/`).
2. Add a link in `docs/README.md`.
3. Use `UPPER_SNAKE_CASE.md` for major docs.
4. After moving or renaming, update links in other docs and in this file.
