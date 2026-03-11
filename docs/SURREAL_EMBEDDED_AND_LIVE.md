# SurrealDB: Embedded (local-first) and Live UI options

This doc outlines how you can use **embedded SurrealDB** (local-first / offline in Tauri) and **live queries** (real-time UI without polling). Both are things ArangoDB doesn’t offer in the same way.

## Current setup

- **Backend** (`back/api`): Connects to SurrealDB over **WebSocket** (`Surreal::new::<Ws>(url)`). SurrealDB runs as a separate service (Docker, etc.).
- **Tauri** (`front/tauri`): Shell around the Yew app; talks to the backend HTTP API. **No DB inside the app.**
- **Real-time**: No live queries; UI refreshes by refetching (e.g. profile summary, bundle).

So today: **remote DB, no embedded engine, no live streams.**

---

## 1. Embedded / Tauri (local-first, offline)

**Goal:** DB inside the app (e.g. local-first, offline-capable).

**What Surreal gives you:**  
Same Surreal API, but the engine runs in-process using a local store:

- **In-memory** (`Mem`) – tests, ephemeral.
- **RocksDB** – file-based, single-node, good for desktop (e.g. Tauri).
- **IndexedDB** – browser; not directly in Tauri, but same idea for “local” in web.

So you can run Surreal **inside** the Tauri (Rust) process with e.g. RocksDB and keep the same SurrealQL and types.

**Ways to use it in this repo:**

| Approach | Where DB runs | Pros | Cons |
|----------|----------------|------|------|
| **A. Tauri-only local** | Embedded Surreal (e.g. RocksDB) in Tauri | True offline, no backend required for local data | Sync to “cloud” Surreal is your job (custom or future Surreal sync) |
| **B. Backend embedded** | Backend uses embedded Surreal (RocksDB) instead of WS | One less service; same API | Backend is stateful; no multi-instance without shared store |
| **C. Hybrid** | Tauri has embedded for offline; backend has remote Surreal | Best of both: offline + central data | Most work: sync logic, conflict handling, when to use which |

**Concrete steps for “embedded in Tauri” (approach A):**

1. **Tauri crate**  
   Add Surreal with a **local** engine, e.g.:
   - `surrealdb = { version = "2", default-features = false, features = ["kv-rocksdb"] }`  
   (or `kv-mem` for a quick test.)

2. **Init at startup**  
   In Tauri’s Rust code (e.g. `main` or a setup function):
   - `let db = Surreal::new::<RocksDb>(path).await?`  
   with `path` = a dir under the app data (e.g. Tauri’s `app_data_dir`).

3. **Use the same API**  
   `db.use_ns(...).use_db(...)`, then your existing SurrealQL (create/select/update, etc.). No WebSocket; it’s in-process.

4. **Sync (optional)**  
   To have “cloud” data too, you’d add a sync layer: e.g. backend (remote Surreal) as source of truth, and periodically or on-event push/pull to the embedded DB (custom protocol or future Surreal features).

**Arango comparison:**  
Arango doesn’t ship an embeddable engine in the same way; you’d run a separate Arango process or use something else (e.g. SQLite) for local. Surreal’s embedded story fits “DB inside the app” cleanly.

---

## 2. Live UI (real-time updates without polling)

**Goal:** UI updates when data changes (e.g. contest results, leaderboard) without the frontend polling.

**What Surreal gives you:**  
**Live queries** – you subscribe to a `LIVE SELECT`; the server pushes changes (create/update/delete) over the same connection. The Rust SDK exposes this as a stream (e.g. `.select("table").live().await` → stream of notifications).  
Works with **WebSocket** and **local/embedded** engines; typically not over plain HTTP.

**Ways to use it in this repo:**

| Approach | Who subscribes | How UI gets updates |
|----------|-----------------|---------------------|
| **A. Backend subscribes, pushes to frontend** | Backend holds a live query to Surreal (WS or embedded) | Backend forwards to the app via WebSockets or SSE (e.g. Actix WebSockets). Frontend only talks to backend. |
| **B. Frontend talks to Surreal directly** | Browser/Tauri app opens a WS (or in-process) to Surreal | UI subscribes to live queries itself. | Requires Surreal reachable from the client (and auth/CORS if browser). |

**Recommended for you: A (backend subscribes).**  
Keeps auth and Surreal behind the API; frontend stays unchanged from a “who do I call?” perspective; you only add a “live” channel (WS or SSE) from backend to frontend.

**Concrete steps for “live contest results” (backend subscribes):**

1. **Backend**  
   - Keep current Surreal WS connection.  
   - In a long-lived task or actor, run e.g.  
     `db.select("resulted_in").live().await`  
     (or a more specific `LIVE SELECT` with filters).  
   - You get a stream of `Notification<Value>` (or your record type).

2. **Broadcast to clients**  
   - When a notification arrives, push it to connected frontends, e.g.:  
     - **Actix WebSocket** (you already have `actix-ws`): each tab has a WS to the backend; backend sends `{ "table": "resulted_in", "action": "CREATE", "data": {...} }`.  
     - Or **SSE**: backend exposes an SSE endpoint; frontend subscribes and receives the same events.

3. **Frontend**  
   - Open a WS (or SSE) to the backend “live” endpoint.  
   - On message, update local state (e.g. contest results, leaderboard) and re-render. No polling.

**Arango comparison:**  
Arango doesn’t have built-in “live query” streams like Surreal. You’d do WebSockets + polling, or change streams, or another stack. Surreal’s live queries give you a single, consistent way to stream changes.

---

## Summary

| Capability | With Surreal (this stack) | With Arango |
|------------|---------------------------|------------|
| **Embedded / local-first** | Yes: same API, embed in Tauri (e.g. RocksDB) or in backend. | No: no embeddable engine; separate process or different local DB. |
| **Live UI** | Yes: `LIVE SELECT` + stream; backend subscribes and pushes via WS/SSE. | Not built-in; you add WS + polling or similar. |

**Minimal next steps if you want to try:**

1. **Embedded (Tauri):** Add `surrealdb` with `kv-rocksdb` to `front/tauri/src-tauri/Cargo.toml`, open `Surreal::new::<RocksDb>(path)` at startup, and run a small SurrealQL script (e.g. create a table, insert, select) to validate. No change to the backend yet.
2. **Live UI:** In the backend, add one `LIVE SELECT` (e.g. on `resulted_in` or `contest`), and an Actix WebSocket (or SSE) endpoint that forwards notifications to the frontend; then have the profile or contest page listen on that channel and update state.

If you say which you want to do first (embedded in Tauri vs live contest results), the next step can be a concrete patch plan (files and code changes) for this repo.
