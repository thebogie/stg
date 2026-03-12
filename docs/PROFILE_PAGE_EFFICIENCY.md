# Profile Page — High-Efficiency Stack Design

A stack-aware design for making the Player Profile page efficient for users and the system, leveraging **Rust**, **shared types**, **Tauri**, **Yew**, **Redis**, and **SurrealDB**.

---

## 1. Goals

- **Fast first paint**: User sees header and core stats as soon as possible.
- **Progressive loading**: Tabs fill as data arrives; no single slow request blocking the shell.
- **Minimal duplicate work**: Backend and DB do not repeat the same queries for the same profile in one load.
- **Repeat visits**: Cache (frontend + backend) so back-navigation and refreshes are cheap.
- **Scalability**: Redis for shared cache across API instances; SurrealDB indexes for fast profile queries.

---

## 2. Stack Roles

| Layer | Role |
|-------|------|
| **Yew (frontend)** | Single `use_profile_data` hook; parallel fetch (summary, achievements, opponents, bundle, ratings, history); in-memory `ProfileCacheContext` for back-navigation; pure tab components over state. |
| **Shared** | Single DTO definitions (`ProfileSummaryDto`, `ProfileBundleDto`, `ProfileOpponentsDto`, etc.) so backend and frontend (and optional Tauri) share the same contract; no drift, no duplicate types. |
| **Backend (Rust/Axum)** | Repository (SurrealDB only) + use case (orchestration + cache-aside). Controller resolves `"me"` and maps errors. All profile endpoints use the same cache layer (Redis or in-memory). |
| **Redis** | Used for both **sessions** (login) and **analytics cache**. Session: one key per session (`session_id → email`, TTL 1h). Profile cache: keys like `analytics:profile_bundle:{player_id}`. Same Redis, different key namespaces; profile is keyed by `player_id` (resolved from session email), not by session_id. |
| **SurrealDB** | Source of truth for analytics; indexed `resulted_in` (`in`, `out`) for fast opponent and stats queries; single `get_opponent_stats_both` used by both bundle and opponents endpoint. |
| **Tauri (optional)** | When running as desktop app: same HTTP API or optional single `invoke('load_profile')` that returns a combined payload to reduce round-trips. |

---

## 3. Backend Efficiency

### 3.1 Cache-aside, per resource

- **Keys**: `profile_summary:{player_id}`, `profile_bundle:{player_id}`, `profile_opponents:{player_id}` (with `analytics:` prefix when using Redis).
- **TTLs**: Summary and bundle short (e.g. 1 min) for freshness; opponents 15 min so Nemesis tab is fast on repeat.
- **Flow**: Use case checks cache → on miss, calls repository (or `tokio::join!` for bundle) → serializes and sets cache → returns.

### 3.2 Avoid duplicate SurrealDB work

- **Opponents**: The frontend requests both `GET .../profile/opponents` and `GET .../profile` (bundle) in parallel. The bundle already runs `get_opponent_stats_both`. So:
  - **Cache `get_profile_opponents`**: First request (opponents or bundle) hits SurrealDB; result is cached under `profile_opponents:{player_id}`.
  - **Prime from bundle**: When building the bundle, after caching the bundle, also write the same opponents data to `profile_opponents:{player_id}`. Then a concurrent or later `GET .../profile/opponents` is a cache hit.
- **Single repo path**: `get_opponent_stats_both` is the only path for opponent stats (used by bundle and by the dedicated opponents endpoint). No separate “who beat me” / “I beat” queries that could double DB load.

### 3.3 Invalidation

- **On contest result write**: Invalidate profile data for affected players so the next load sees fresh stats. Explicitly remove:
  - `profile_bundle:{player_id}`
  - `profile_summary:{player_id}`
  - `profile_opponents:{player_id}`
  in addition to any pattern-based invalidation for other player-scoped keys (e.g. `player_stats`, `player_achievements`). This keeps Redis SCAN patterns simple and ensures profile keys are always cleared when results change.

### 3.4 Parallelism in Rust

- **Bundle**: Use case runs `tokio::join!` for display_label, stats, achievements, game_performance, trends, and opponents. One HTTP request for the bundle triggers one coordinated SurrealDB + cache layer workload.
- **Timeouts**: Opponents (and bundle’s opponent fetch) are wrapped in a timeout (e.g. 15s) so a slow or hanging query does not block the whole response.

---

## 4. SurrealDB Efficiency

- **Indexes**: Use `resulted_in_in` and `resulted_in_out` (see `docs/surreal-indexes-optional.surql`) so `WHERE \`in\` = type::record('player', $key)` and `WHERE \`out\` INSIDE $contest_ids` use indexes.
- **Opponent stats**: Two-step pattern (my contest IDs → all rows for those contests) avoids heavy nested subqueries and full scans; both steps use indexed lookups.
- **No N+1**: Opponent display names are fetched with a single `WHERE id INSIDE $player_ids` after aggregating opponent IDs in memory.

---

## 5. Frontend Efficiency

- **One hook**: `use_profile_data` owns all profile server state (summary, achievements, opponents, bundle, ratings, history). Tabs are pure UI; they do not fetch.
- **Parallel requests**: Summary, achievements, opponents, and (bundle + ratings + history) are started together. Summary is the critical path for “loading = false” and first paint.
- **Bundle overwrites**: When the full bundle returns, it overwrites tab state (including opponents) so the UI is consistent and the in-memory cache can store one bundle for back-navigation.
- **ProfileCacheContext**: 5-minute TTL; returning to the profile page hydrates from cache and skips refetch when the entry is still fresh.
- **No duplicate instrumentation**: No debug logging or external ingest calls in the hot path.

---

## 6. Redis Usage

- **Sessions (login)**: We **do** use Redis for login. On login, `RedisSessionStore::set_session(session_id, email)` stores `session_id → email` with a 1h TTL. Auth middleware validates the Bearer token by doing `GET session_id` and resolving the user from the stored email. So the “login slot” is that key; today it holds only the email (or equivalent identifier), not profile data.
- **Analytics cache**: The analytics controller is built with `RedisAnalyticsCache` (same Redis client as sessions). Profile data is cached under keys like `analytics:profile_bundle:{player_id}`, `analytics:profile_summary:{player_id}`, `analytics:profile_opponents:{player_id}`. So profile cache is in Redis, keyed by **player_id** (we resolve `"me"` from the session’s email to get `player_id`). “When Redis is enabled” in the doc meant: whenever the app runs with Redis configured (which it does in normal dev/prod), both session store and analytics cache use Redis; the only alternative is in-memory analytics cache in tests or minimal setups.
- **Why not store profile inside the session key?** We could extend the session value from `email` to e.g. a JSON blob like `{ "email": "...", "player_id": "...", "profile_summary": {...} }` so one Redis GET returns auth + cached profile. Trade-offs: (1) session payload grows and can get stale; (2) when we invalidate profile (e.g. after a new contest result), we’d need to invalidate or refresh that session value—we don’t currently keep a reverse index (player_id → session_ids), so we’d either add that, accept staleness until session TTL, or refetch profile on each request. The current design keeps the session small (email only) and caches profile by `player_id` in separate keys, so invalidation is a simple DEL on `analytics:profile_*:{player_id}` and any request with that player_id gets fresh data after the next load. If you want to try “cache in login slot,” a reasonable first step is to store only `player_id` (and maybe a minimal summary) in the session and keep the full profile bundle in the existing analytics keys.
- **Failure mode**: If Redis is down, session validation fails (user cannot be authenticated) and cache get/set fails gracefully for analytics; profile still loads from SurrealDB on cache miss.

---

## 7. Tauri (Optional)

- **Browser**: Frontend uses `authenticated_get` and the same API base URL; no change.
- **Desktop**: Same fetch-based flow is sufficient. Optionally, a Tauri command can call the backend once and return a combined payload (e.g. summary + opponents + achievements + bundle + ratings + history) so the desktop app makes one round-trip instead of six. The backend can support a single “profile/init” endpoint that returns multiple DTOs in one JSON object, or the command can call existing endpoints in sequence and merge; in both cases shared DTOs keep the contract consistent.

---

## 8. Summary of Implemented Behaviors

- **Backend**: `get_profile_opponents` uses cache (Redis or in-memory); on bundle build, opponents cache is primed; `invalidate_player_cache` explicitly removes profile_bundle, profile_summary, and profile_opponents.
- **SurrealDB**: Indexes on `resulted_in`; single `get_opponent_stats_both` path; timeouts to avoid hangs.
- **Frontend**: Progressive + parallel load; single hook; cache context for back-navigation; debug instrumentation removed.
- **Redis**: Used for all analytics profile keys when configured; TTLs and invalidation keep data fresh without unnecessary DB load.

This yields a high-tech, efficient Profile page that uses the advantages of Rust (parallelism, type safety, explicit caching), shared DTOs (one contract), Yew (single state source, parallel async), Redis (shared, persistent cache), and SurrealDB (indexed, single-query patterns) without blocking first paint or doing duplicate work across the stack.
