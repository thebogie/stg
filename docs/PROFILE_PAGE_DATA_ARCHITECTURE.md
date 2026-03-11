# Profile Page — Holistic Data Architecture & Industry-Standard Rust Patterns

## 1. Current State (Holistic View)

### Tabs and Their Data

| Tab | Data | Source today | Heaviness |
|-----|------|--------------|-----------|
| **Overall Stats** | display_label, player_stats (contests, wins, streaks, skill_rating) | Summary + bundle | Light (summary) / full (bundle) |
| **Ratings** | Glicko ratings, rating history | Separate: `/ratings/current`, `/ratings/history` | Medium |
| **Achievements** | badges, progress | Dedicated fetch + bundle | Light (dedicated) / full |
| **Nemesis** | opponents_who_beat_me | Dedicated `/profile/opponents` + bundle | Light (dedicated) / full |
| **Owned** | opponents_i_beat | Same opponents response + bundle | Light / full |
| **Game Performance** | per-game stats (plays, wins, placement) | Full bundle only | Heavy |
| **Trends** | monthly performance_trends + filters | Full bundle + optional filtered API | Heavy |
| **Comparison** | on-demand head-to-head | On-demand when tab/modal used | On-demand |
| **Settings** | local/prefs | No server | — |

### Current Load Strategy

- **First paint:** Summary (display + stats) → `loading = false` so the shell appears fast.
- **Parallel:** Summary, achievements, opponents, and (in one join) full bundle + ratings + history.
- **Full bundle:** When it arrives, it overwrites all tab state (cache, then used for back-navigation).
- **Cache:** In-memory `ProfileCacheContext` with TTL; returning to profile can hydrate from cache and skip refetch.

So we already use: **progressive loading** (summary first), **parallel requests** (summary, achievements, opponents, bundle+ratings+history), **cache-aside** (context + backend cache), and **single source of truth** (one hook, one set of handles).

---

## 2. Industry-Standard Rust Patterns for “Multi-Tab Profile” Data

### 2.1 Backend: Data Pull and Calculation

**Pattern: Repository + Use Case + Cache (no business logic in controller)**

- **Repository:** Single place for SurrealDB (or any DB). Returns domain/DTO types. No HTTP, no “profile” concept—only “get player stats”, “get opponent stats”, etc.
- **Use case:** Orchestrates repos and caching. Knows about “profile bundle” and “profile summary” and “profile opponents”. Runs independent queries in parallel (`tokio::join!`), then assembles DTOs. All “calculation” that is just aggregation belongs here or in the repo (e.g. win rate from wins/contests).
- **Controller:** Resolves `"me"` → player_id, calls use case, maps errors to HTTP. No business logic.

**Where to put calculation:**

- **In repository:** When it’s a single query or a small, query-scoped aggregation (e.g. win rate from `wins_against_me` and `contests_played`). Keeps use case thin.
- **In use case:** When you combine several repo results (e.g. “profile bundle” = join of 6 repo calls). Or when you apply business rules (e.g. “skill_rating from Glicko if present, else from stats”).
- **Not in controller:** Controllers should not compute stats or merge DTOs.

**Caching (industry standard: cache-aside):**

- **Key:** Per-resource (e.g. `profile_bundle:{player_id}`, `profile_summary:{player_id}`, `profile_opponents:{player_id}`).
- **TTL:** Short for bundle/summary (e.g. 5 min) so repeat visits are fast but data is fresh.
- **Place:** Use case checks cache before calling repo; writes through on success. Repository stays cache-agnostic.

**Parallelism:**

- One heavy “bundle” that runs 6 repo calls in parallel (`tokio::join!`) is the right Rust pattern. No need for 6 separate HTTP endpoints for that unless you want to **also** support lightweight endpoints for first paint (like summary and opponents).

### 2.2 Frontend: Data Pull and State

**Pattern: Single source of truth + granular loading + parallel fetch**

- **One hook** (`use_profile_data`) that owns:
  - All profile-related server state (summary, achievements, opponents, bundle, ratings, history).
  - Global `loading` / `error` for “page ready” (e.g. summary arrived).
  - Per-resource loading where useful (e.g. `glicko_loading`, `rating_history_loading`).
- **No duplicate fetches:** Tabs don’t fetch on their own; they receive props from the hook. Comparison and head-to-head modals are the exception (on-demand when user opens them).

**Progressive / parallel fetch (what you have):**

1. **Critical path (first paint):** One fast request (e.g. summary) and set `loading = false` when it returns so the shell and header render.
2. **Above-the-fold or likely tabs:** Start requests in parallel (e.g. summary, achievements, opponents) so those tabs can show as soon as their response arrives.
3. **Full bundle:** Run in parallel with ratings/history; when it arrives, overwrite tab state. This gives cache and consistency without blocking first paint.
4. **Optional: Tab-on-demand.** For tabs that are heavy and rarely opened (e.g. Trends with filters), you can fetch only when the user switches to that tab. That’s a trade-off: simpler model is “load everything in parallel” (current); “load on tab switch” reduces initial work but can add perceived delay when opening the tab.

**Rust/Yew-specific:**

- **Async:** Use `spawn_local` for fire-and-forget or “then update state” flows. Use `join3`/`join_all` when you need to wait for several requests and then apply state once.
- **State:** `UseStateHandle<Option<T>>` per resource is standard. “Loading” can be “None and no error yet” or an explicit `loading: bool` per resource.
- **Context:** `ProfileCacheContext` for cross-navigation cache is the right idea; one provider, consumers read from it so revisiting the profile can skip refetch when cache is fresh.
- **Errors:** One global `error: Option<String>` for “profile failed” plus optional per-resource errors (e.g. ratings) keeps UI simple.

**No “React Query” in Rust (yet):** There is no dominant standard like React Query in the Yew ecosystem. The pattern you have—one hook, parallel requests, cache in context, granular state—is the industry-standard approach in Rust frontends: **explicit state and explicit fetch**, with cache-aside and progressive loading.

### 2.3 Recommended Contract for “All Tabs”

**Backend:**

- **Lightweight, for first paint and key tabs:**
  - `GET .../profile/summary` → display + stats (2 light queries).
  - `GET .../profile/opponents` → opponents_who_beat_me + opponents_i_beat (Nemesis + Owned).
  - `GET .../achievements` → achievements (Achievements tab).
- **Heavy, for full tab set and cache:**
  - `GET .../profile` → full bundle (display, stats, achievements, game_performance, trends, opponents). Run 6 repo calls in parallel in the use case; cache result.
- **Separate (different domain):**
  - Ratings: `GET /api/ratings/current`, `GET /api/ratings/history?scope=global`.
- **On-demand:**
  - Head-to-head, comparison, filtered trends when user opens tab/modal.

**Frontend:**

- On profile load (or cache miss): start **in parallel** summary, achievements, opponents, and (bundle + ratings + history) in one join.
- Set `loading = false` when summary returns (or when cache hydrates).
- When each response returns, set the corresponding state; when the full bundle returns, overwrite all tab state (and update cache).
- Tabs are pure view: they receive `Option<T>` (and maybe a loading flag) and render content or skeleton/empty/error.

---

## 3. Summary: Best Practice in One Sentence

**Backend:** One use case orchestrates all profile data; repository does only DB access; run independent queries in parallel (`tokio::join!`); cache per resource in the use case; expose a small set of endpoints (summary, opponents, achievements, full bundle) so the frontend can do progressive + parallel loading.

**Frontend:** One hook owns all profile server state; fetch summary + achievements + opponents + (bundle, ratings, history) in parallel; set loading false on first critical response (summary); fill tabs as responses arrive; cache bundle in context for repeat visits; tabs are pure UI over that state.

This is the same pattern used in production Rust backends (Axum/Actix + repo + use case + cache) and in Yew apps (single data hook + parallel async + context cache): **explicit, testable, and predictable** without relying on a framework-specific data layer.
