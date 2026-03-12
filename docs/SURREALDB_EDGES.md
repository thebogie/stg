# SurrealDB record IDs and edges — standard convention

We follow **SurrealDB’s recommended approach**: all record references are stored as **record ids (record id type)**, not plain strings. That keeps comparisons and graph-style queries consistent and avoids type mismatches.

---

## Rule: Record id type everywhere (SurrealDB v3)

| Where            | What              | Format in DB / in queries |
|------------------|-------------------|----------------------------|
| Document `id`    | player, game, contest, venue, etc. | Stored and compared as **record id**. In SurrealQL (v3): `type::record("table", "key")` or `table.id`. |
| Edge `out` / `in` | played_at, played_with, resulted_in | Stored and compared as **record id**. Same as above. |
| API / app code   | IDs in DTOs       | Use string form `table/key` or `table:key` at boundaries; normalize with `replace("table:", "table/")` etc. as needed. |

So in the database:

- **Documents**: `id` is a record id (e.g. `player:2025041711441879994340500`).
- **Edges**: `out` and `in` are record ids (e.g. `contest:...`, `player:...`, `venue:...`, `game:...`).

No mixing: we do **not** store record references as plain strings in the DB.

---

## ArangoDB → SurrealDB converter (arango-to-surreal)

The tool in **tools/arango-to-surreal** produces `.surql` that matches this convention.

- **Document tables** (player, game, venue, contest, rating_latest, rating_history, …):  
  - Row `id`: `type::record("table", "key")`.  
  - **Reference fields** (`contest.creator_id`, `rating_*.player_id`, `rating_*.scope_id` when non-null): values like `"player/123"` or `"game:456"` are emitted as `type::record("player", "123")` / `type::record("game", "456")` so the backend’s `WHERE player_id = type::record('player', $key)` etc. match.  
  - Duplicate keys that differ only by case (e.g. `createdat` and `createdAt`) are deduped; the converter keeps the camelCase form.

- **Edge tables** (played_at, played_with, resulted_in):  
  - Row `id`: `type::record("played_at", "key")` (etc.).  
  - `out` and `in`: `type::record("contest", "key")`, `type::record("player", "key")` (or the correct table for that edge).

So:

- **New imports**: Run the converter on your Arango dump and import the generated `.surql`. No extra migration is needed for record id types.
- **Old imports**: If you have an existing DB where edges (or document ids) were imported as **strings**, run the one-time migration below so everything is Thing type.

---

## Backend query patterns

- **By key (param from API):**  
  `WHERE id = type::record('player', $key)`  
  and for edges:  
  `WHERE \`in\` = type::record('player', $key)`,  
  `WHERE \`out\` = type::record('contest', $key)`.

- **Correlated with another table:**  
  `FROM player WHERE (SELECT count() FROM resulted_in WHERE \`in\` = player.id) > 0`  
  Here `player.id` and `resulted_in.in` are both record ids, so they compare directly. No `string::concat()`.

- **Binding a record id:**  
  Pass the key (string) and use `type::record('table', $key)` in the query. Do not store or compare raw strings for record references in the DB.

---

## Edge table convention

- **`out`** = source vertex (the “from” side).
- **`in`** = target vertex (the “to” side).

So for each edge row, the relationship is **out → in**.

| Table          | `out`    | `in`    | Meaning                          |
|----------------|----------|---------|----------------------------------|
| played_at      | contest  | venue   | Contest was played at venue      |
| played_with    | contest  | game    | Contest included game            |
| resulted_in    | contest  | player  | Contest had player result (place, result) |

---

## One-time migration: string → record type (existing DBs)

If your database was imported from an **older** `.surql` that wrote edge `out`/`in` (or document `id`) as **strings**, run the migration below once so everything is record id type (SurrealDB v3: `type::record`). After that, the backend and converter behave consistently.

Use the same **namespace** and **database** as your app (e.g. in Surrealist or `surreal sql`).

### 1. played_at (`out` = contest, `in` = venue)

```sql
UPDATE played_at SET
  out = type::record('contest', string::replace(string::concat(out), 'contest:', '')),
  in  = type::record('venue',  string::replace(string::concat(in),  'venue:',  ''));
```

### 2. played_with (`out` = contest, `in` = game)

```sql
UPDATE played_with SET
  out = type::record('contest', string::replace(string::concat(out), 'contest:', '')),
  in  = type::record('game',    string::replace(string::concat(in),  'game:',    ''));
```

### 3. resulted_in (`out` = contest, `in` = player)

```sql
UPDATE resulted_in SET
  out = type::record('contest', string::replace(string::concat(out), 'contest:', '')),
  in  = type::record('player',  string::replace(string::concat(in),  'player:',  ''));
```

`string::concat()` works for both string and Thing columns, so the same statements work whether the column is currently string or already Thing.

### Check after migration

```sql
SELECT out, in FROM played_at LIMIT 1;
SELECT out, in FROM resulted_in LIMIT 1;
```

You should see record id values (e.g. `contest:⟨...⟩`, `player:⟨...⟩`), not plain quoted strings.

---

## References

- [SurrealDB Record IDs](https://surrealdb.com/docs/surrealql/datamodel/ids)
- [SurrealDB Graph relations](https://surrealdb.com/docs/surrealdb/reference-guide/graph_relations)
- Project: **tools/arango-to-surreal** (converter), **back/api** (queries)
