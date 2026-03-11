# SurrealDB migration status

## Completed

- **Config**: SURREAL_* env vars, DatabaseConfig.ns
- **Cargo**: surrealdb, no arangors
- **main.rs**: Connects SurrealDB (Ws), signin Root, use_ns/use_db, passes Db
- **db.rs**: `pub type Db = Surreal<Client>`
- **error.rs**: From<surrealdb::Error>, test_from_surrealdb_error
- **health.rs**: Db, check_database with `db.query("SELECT 1")`, detailed_health_check and check_scheduler use Db and RatingsScheduler (no generic)
- **health tests**: Use SurrealDB (Surreal::new::<Ws>, signin Root, use_ns/use_db); skip if connection fails
- **ratings**: repository (Db, SurrealQL), usecase (Db), scheduler (no type param), controller (Db, configure_routes(db, scheduler, redis))
- **auth**: AdminAuthMiddleware uses Db; player admin check via SurrealQL
- **player/repository.rs**: Db; find_by_email, find_by_id, find_many_by_ids, search_players, create, update, find_by_handle with SurrealQL; IDs normalized to player/key in API

## In progress / TODO

- **game/repository.rs**: Replace Database<ReqwestClient> with Db, all AQL → SurrealQL
- **venue/repository.rs**: Same (Db, SurrealQL)
- **contest/repository.rs**: Db; create contest + played_at, played_with, resulted_in (in/out record ids)
- **analytics/repository.rs, controller.rs, usecase.rs**: Db, all AQL → SurrealQL
- **client_analytics/repository.rs, controller.rs, usecase.rs**: Db
- **migration/timezone_migration.rs**: Db
- **Tests**: Replace arangors/Database<ReqwestClient> with Db or skip (ratings_tests, scheduler_tests, usecase_tests, analytics, migration tests, etc.)
- **Docker/deploy**: Align with ShelfToadFlip (docker-compose.yml, docker-compose.ci.yml, docker-compose.local.yml, _deploy/ci-local.sh, _deploy/start-stack-local.sh)

## Data model (Surreal)

- Tables: contest, game, player, venue, rating_latest, rating_history
- Edge-like tables: resulted_in, played_at, played_with with `in` and `out` record links (use backticks if reserved)
- Record IDs: player:key, contest:key, game:key, venue:key; API exposes player/key etc.
