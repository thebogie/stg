# SurrealDB code migration checklist

This tracks the backend switch from ArangoDB (arangors) to SurrealDB. The backend **will not compile** until every module below is migrated (arangors has been removed).

## Done
- [x] **Config** (`backend/src/config.rs`): `DatabaseConfig` has `ns`; `url`/`name`/auth load from `SURREAL_*` (with `ARANGO_*` fallback). Default URL `http://localhost:50001`.
- [x] **Cargo** (`backend/Cargo.toml`): `surrealdb` added; `arangors` removed.
- [x] **main.rs**: Connects to SurrealDB (Ws), signin with Root, `use_ns().use_db()`, passes `Db` to all repos and routes.
- [x] **db.rs**: New module; `pub type Db = Surreal<Client>` so all code uses `backend::db::Db`.

## Your next step: migrate the rest

Replace **every** use of `arangors` with SurrealDB in these files (use `Db` and SurrealQL / `.select()`, `.create()`, `.query()`, etc.):

| Area | Files to change |
|------|------------------|
| **Error / health** | `error.rs` (`From<surrealdb::Error>`), `health.rs` (DB health with Surreal) |
| **Player** | `player/repository.rs` |
| **Game** | `game/repository.rs` |
| **Venue** | `venue/repository.rs` |
| **Contest** | `contest/repository.rs` (contest + played_at, played_with, resulted_in) |
| **Ratings** | `ratings/repository.rs`, `ratings/scheduler.rs`, `ratings/controller.rs`, `ratings/usecase.rs` |
| **Analytics** | `analytics/repository.rs` (largest – many AQL queries), `analytics/controller.rs`, `analytics/usecase.rs` |
| **Client analytics** | `client_analytics/repository.rs`, `client_analytics/controller.rs`, `client_analytics/usecase.rs` |
| **Auth** | `auth.rs` |
| **Migration** | `migration/timezone_migration.rs` |
| **Tests** | `*_tests.rs` that mock or use `Database<C>` / `ReqwestClient` |

Pattern: take `Db` instead of `Database<ReqwestClient>` or `Database<C>`; replace AQL with SurrealQL (e.g. `db.query("SELECT * FROM player WHERE id = $id").bind(("id", id)).await`) or SDK methods (e.g. `db.select(("player", key)).await`). Normalize IDs at the boundary: Surreal uses `player:key`, API/shared models use `player/key`.

## ID format
- **Stored in Surreal**: record ids like `player:123`, `contest:456` (converter output).
- **API / shared models**: Keep `player/123`, `contest/456` for compatibility. Repositories normalize at boundary: when reading from Surreal, map `player:key` → `player/key` for the `id` field; when writing, map `player/key` → record id `player:key`.

## Env (for reference)
- `SURREAL_URL` (default dev: `http://localhost:50001`)
- `SURREAL_NS`, `SURREAL_DB` (default same as current DB name, e.g. `stg_rd`)
- `SURREAL_USER`, `SURREAL_PASSWORD` (default dev: root/root)
