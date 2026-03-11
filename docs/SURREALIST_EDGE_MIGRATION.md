# Fix SurrealDB edge type via Surrealist (no backend rebuild)

If **contest list**, **contest detail**, **leaderboard**, or **analytics** are empty or wrong because edge `out`/`in` were imported as **strings** instead of record ids (Thing), run the edge migration once. No backend restart needed.

---

## Standard we follow

We store all record references as **record id (Thing)** in SurrealDB. See **docs/SURREALDB_EDGES.md** for the full convention, converter behavior, and backend query patterns.

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
| Contest detail (no venue, games, outcomes) | `WHERE out = type::thing($contest_rid)` and similar match. |
| Leaderboard / analytics empty | `resulted_in.in = player.id` and other edge comparisons work. |
| **Achievements empty or 500** | Achievements need `resulted_in` (and optionally `played_with` / `played_at`) with record id `out`/`in`. Run this migration if edges were ever imported as strings. |

---

## After running

- Backend restart is **not** required.
- New imports from **arango-to-surreal** already emit `type::thing()` for edges, so no migration is needed for freshly imported data.

---

## Verify

In Surrealist:

```sql
SELECT out, in FROM played_at LIMIT 1;
SELECT out, in FROM resulted_in LIMIT 1;
```

You should see record id values (e.g. `contest:⟨...⟩`, `player:⟨...⟩`), not plain quoted strings.
