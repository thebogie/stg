# Fix SurrealDB edge type via Surrealist (no backend rebuild)

**One-time move from ArangoDB:** If you are doing a **fresh** move (restart SurrealDB, run **arango-to-surreal** on your dump, then import the generated `.surql`), you **do not need this migration**. The converter already emits `type::record()` for edge `in`/`out` (SurrealDB v3), so edges land as record ids. Use this doc only if you already have a SurrealDB that was imported with **string** edges (e.g. older export or different import path).

If **contest list**, **contest detail**, **leaderboard**, or **analytics** are empty or wrong because edge `out`/`in` were imported as **strings** instead of record ids, run the edge migration once. No backend restart needed.

**SurrealDB v3:** We use `type::record(...)` in SurrealQL (v2 used `type::thing`). The migration script and backend use the v3 form.

---

## Standard we follow

We store all record references as **record id type** in SurrealDB. See **docs/SURREALDB_EDGES.md** for the full convention, converter behavior, and backend query patterns.

---

## Run the migration

**Option A — Run the script (recommended)**  
Use the same namespace and database as your app.

```bash
surreal sql --endpoint <url> --ns <ns> --db <db> -f docs/surreal-migrate-edge-strings-to-things.surql
```

**Option B — Run in Surrealist**  
Open the Query tab, select the correct namespace and database, then run the contents of **docs/surreal-migrate-edge-strings-to-things.surql** (the three UPDATE statements for played_at, played_with, resulted_in).

---

## What this fixes

| Issue | Fix |
|--------|-----|
| Contest list empty | Edge `out`/`in` become record ids so `id INSIDE (SELECT out FROM played_at)` etc. match. |
| Contest detail (no venue, games, outcomes) | `WHERE out = type::record($contest_rid)` and similar match. |
| Leaderboard / analytics empty | `resulted_in.in = player.id` and other edge comparisons work. |
| **Achievements empty or 500** | Achievements need `resulted_in` (and optionally `played_with` / `played_at`) with record id `out`/`in`. Run this migration if edges were ever imported as strings. |

---

## After running

- Backend restart is **not** required.
- New imports from **arango-to-surreal** already emit `type::record()` (SurrealDB v3) for edges, so no migration is needed for freshly imported data.

---

**See also:** **docs/SURREALDB_EDGE_QUERY_AUDIT.md** for a full list of edge/ID query call sites and safe patterns (ArangoDB legacy → SurrealDB).

---

## Verify

In Surrealist:

```sql
SELECT out, in FROM played_at LIMIT 1;
SELECT out, in FROM resulted_in LIMIT 1;
```

You should see record id values (e.g. `contest:⟨...⟩`, `player:⟨...⟩`), not plain quoted strings.
