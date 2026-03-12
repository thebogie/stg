# SurrealDB indexes for search and order

**SurrealDB v3.** Record-id lookups use `type::record('table', $key)`. Indexes in SurrealDB speed up **WHERE** filters and can help **ORDER BY** (though as of recent versions, ORDER BY does not always use indexes; use `EXPLAIN` to verify).

## How it helps

- **Primary lookups by record id** (`WHERE id = type::record('table', $key)` in SurrealDB v3) already use the primary index; no extra index needed.
- **Filters on other columns** (e.g. `WHERE email = $email`, `WHERE \`in\` = type::record('player', $key)`) do a table scan unless you add a **secondary index** on that column.
- **ORDER BY** on indexed columns can be faster in some cases; SurrealDB is still improving this. Prefer filtering first and limiting result set size.

Define indexes with:

```sql
DEFINE INDEX index_name ON TABLE table_name COLUMNS field1, field2;
```

Use `EXPLAIN` on a query to see whether it uses "Iterate Index" (good) or "Iterate Table" (full scan).

## Indexes we use

| Table         | Index                      | Purpose |
|---------------|----------------------------|--------|
| contest       | contest_start, contest_stop | Time-range and ORDER BY start/stop |
| rating_latest | rating_latest_scope_player | Lookup by scope_type, player_id, scope_id |
| player        | (see below)                | Lookup by email; search by handle/email |
| resulted_in   | (see below)                | Lookup by player (\`in\`) or contest (\`out\`) |
| played_at     | (see below)                | Lookup by contest (\`out\`) or venue (\`in\`) |
| played_with   | (see below)                | Lookup by contest (\`out\`) or game (\`in\`) |
| rating_history| (see below)                | Lookup by player_id + scope; ORDER BY period_end |

## Applying indexes

1. **New conversion from ArangoDB**: The schema emitted by `tools/arango-to-surreal` (with `--schema`) includes all indexes above: contest (start/stop), rating_latest (scope_player), player (email), rating_history (player_scope, period), and edge tables (resulted_in, played_at, played_with on `in`/`out`). Apply that schema to an empty DB before running the INSERTs.

2. **Existing DB** (converted before these indexes were added): Run the statements in `docs/surreal-indexes-optional.surql` against your namespace/database (e.g. in Surrealist or via the SurrealDB CLI). Use `IF NOT EXISTS` to avoid errors if an index already exists.

3. **Verify**: Run a representative query with `EXPLAIN` and check for "Iterate Index".

## Caveats

- **Write cost**: Every index is updated on INSERT/UPDATE/DELETE. Add indexes only for columns you filter or order by often.
- **ORDER BY**: SurrealDB may not always use an index for ORDER BY; if a query is slow, try reducing the result set with WHERE/LIMIT first.
- **Functions in WHERE**: `WHERE string::lowercase(email) = ...` may not use an index on `email`; the engine might still scan. For exact lookups, prefer indexing the column and testing with EXPLAIN.
