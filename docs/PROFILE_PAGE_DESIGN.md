# Player Profile Page — Tauri-Standard Data Flow & SurrealDB Patterns

## Overview

The Player Profile page is the single place for a player to see their stats, ratings, achievements, nemeses, game performance, trends, and comparison. Data is provided in a **Tauri-standard way**: one clear contract and one load path, whether the app runs in the browser (fetch) or in the Tauri desktop shell (optional invoke).

Each tab is backed by SurrealDB with industry patterns: typed record IDs (`type::thing('player', $key)`), schemafull tables/edges, no table aliases in SurrealQL, repository layer, and scalar extraction helpers.

---

## Data Contract (Single Source of Truth)

### Profile bundle (one HTTP request)

- **Endpoint:** `GET /api/analytics/players/{player_id}/profile` (or `me` for current user)
- **Response:** `ProfileBundleDto` with:
  - `display_label` — handle or name for the header
  - `player_stats` — Overall Stats tab
  - `achievements` — Achievements tab
  - `game_performance` — Game Performance tab
  - `performance_trends` — Trends tab (last 12 months)
  - `opponents_who_beat_me` — Nemesis tab
  - `opponents_i_beat` — Owned tab

### Lightweight (first paint / key tabs)

- **Summary:** `GET .../profile/summary` — display + stats (first paint).
- **Opponents:** `GET .../profile/opponents` — Nemesis + Owned tabs (fetched in parallel with bundle).
- **Achievements:** `GET .../achievements` — Achievements tab.

### Supplementary (same load or on-demand)

- **Ratings:** `GET /api/ratings/current`, `GET /api/ratings/history?scope=global` — Ratings tab
- **Head-to-head:** `GET /api/analytics/player/head-to-head/{opponent_id}` — Nemesis/Owned contest modal
- **Trends (filtered):** `GET /api/analytics/player/performance-trends?game_id=…&venue_id=…` — Trends tab filters
- **Comparison:** separate flow (compare two players)

Settings tab is local/preferences (no SurrealDB).

**Holistic data strategy:** See [PROFILE_PAGE_DATA_ARCHITECTURE.md](PROFILE_PAGE_DATA_ARCHITECTURE.md) for industry-standard Rust patterns (progressive + parallel fetch, cache-aside, single source of truth).

---

## Tab → Backend Data Mapping (SurrealDB)

| Tab | Data source | Repository method(s) | Notes |
|-----|-------------|------------------------|-------|
| **Overall Stats** | Profile bundle `player_stats` | `get_player_display_label`, `get_player_stats` | Aggregates from `resulted_in`, `contest`; scalar extraction via helpers |
| **Ratings** | `/api/ratings/current`, `/api/ratings/history` | Ratings module (Glicko2), not analytics repo | |
| **Achievements** | Profile bundle `achievements` | `get_player_achievements` | Achievement definitions + player progress |
| **Nemesis** | Profile bundle `opponents_who_beat_me` | `get_players_who_beat_me` | Opponents with more wins vs. me; `resulted_in` + contest |
| **Owned** | Profile bundle `opponents_i_beat` | `get_players_i_beat` | Opponents I beat more; same pattern |
| **Game Performance** | Profile bundle `game_performance` | `get_my_game_performance` | Per-game stats; `resulted_in` JOIN `played_with` JOIN `contest` |
| **Trends** | Profile bundle `performance_trends` + optional filtered endpoint | `get_my_performance_trends` | Monthly aggregates; `resulted_in` JOIN `contest` |
| **Comparison** | On-demand | `get_player_stats`, `get_head_to_head_record` (per player) | Compare two players |
| **Settings** | Local / Tauri | — | No SurrealDB |

---

## SurrealDB Industry Patterns (Backend)

- **Record IDs:** Use `type::thing('player', $key)` (and `contest`, `game`, `venue`) with raw key from `record_id_to_key()` / `player_id_to_key()`. Canonical string format in app/DTOs is `"table/key"`; for `INSIDE $ids` bindings use `"table:key"`. See **docs/SURREALDB_ID_CONVENTIONS.md** for the full project standard.
- **No table/edge aliases:** SurrealQL uses full table names (e.g. `FROM resulted_in`, `INNER JOIN contest ON contest.id = resulted_in.\`out\``). No `FROM resulted_in result` or `AS result`.
- **Scalar extraction:** Use shared helpers (`scalar_i64`, `scalar_f64`) for `count()`, `math::*` and other aggregates that may return objects.
- **Typed rows:** Deserialize into structs with `Option<Thing>` for record IDs; use `thing_to_record_id()` to get `"table/key"` strings.
- **Errors:** `map_err` on `.take(0)` and propagate; no `unwrap()` in repo.
- **Schema:** Tables and edges are schemafull; record IDs are strongly typed in the app.

---

## Frontend: Tauri-Standard Single Load

- **Browser:** One initial load: `GET .../profile` + `GET /api/ratings/current` + `GET /api/ratings/history?scope=global`. All tab data (except Comparison and filtered Trends) comes from this.
- **Tauri:** When running inside Tauri, the frontend calls `invoke('get_app_config')` on load to get the API base URL; then all requests use that base (see `front/web/src/tauri.rs`, `config.rs`, `components/config_loader.rs`). Optionally a Tauri command could call the backend (e.g. `get_player_profile`) so the app has one code path for “load profile.”
- **State:** A single profile data hook or context holds: loading, error, bundle, ratings, rating history. Tabs are pure UI over this state.

---

## File References

- **Contract (DTOs):** `shared/src/dto/analytics.rs` — `ProfileBundleDto`, `PlayerStatsDto`, `PlayerAchievementsDto`, `GamePerformanceDto`, `PerformanceTrendDto`, `PlayerOpponentDto`.
- **Backend use case:** `back/api/src/analytics/usecase.rs` — `get_profile_bundle()` runs repo methods in parallel.
- **Backend repo:** `back/api/src/analytics/repository.rs` — all `get_*` methods above; SurrealQL follows no-alias, type::thing, scalar helpers.
- **Backend controller:** `back/api/src/analytics/controller.rs` — `get_profile_bundle`, ratings routes, head-to-head, performance-trends.
- **Frontend:** `front/web/src/pages/profile.rs` — Profile page; can use a single `use_profile_data` hook that returns bundle + ratings + history.
