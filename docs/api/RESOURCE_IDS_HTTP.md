# HTTP paths vs JSON record IDs

This complements [`SURREALDB_ID_CONVENTIONS.md`](../SURREALDB_ID_CONVENTIONS.md) (canonical **`table/key`** in app code and JSON).

## What clients should send

| Layer | Format | Example |
|--------|--------|--------|
| **JSON** `_id` / DTO ids | Canonical `table/key` | `"game/550e8400-e29b-41d4-a716-446655440000"` |
| **URL path** for `GET/PUT/DELETE` `/api/games/{id}` and `/api/venues/{id}` | **Raw record key only** (no slash) | `/api/games/550e8400-e29b-41d4-a716-446655440000` |

The web app builds paths this way (`front/web/src/api/games.rs`, `venues.rs`): strip an optional `game/` or `venue/` prefix from a DTO id, then append the key as a **single path segment**.

## Why not `…/game/<uuid>` as two segments?

Standard routing uses **one** dynamic segment per resource. Putting `game/` inside the path would require an extra path level or percent-encoding; the supported contract is **key-only in the path**, canonical id in JSON.

## Server behavior

`canonical_id_from_http_path_param` in `back/api/src/surreal_helpers.rs` maps the path segment to canonical `table/key` for repository lookups. If a client sends a **single** segment that already looks like `table/key` or `table:key` (unusual), it is normalized the same way as JSON ids.
