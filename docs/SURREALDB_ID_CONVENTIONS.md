# SurrealDB ID Conventions — Project Standard

This document defines how we handle **record IDs** (record id vs string) across the codebase so we avoid the mixing that causes subtle bugs. It aligns with SurrealDB’s model and gives one consistent pattern for queries, bindings, and APIs.

**SurrealDB v3:** Use `type::record('table', $key)` in SurrealQL (v2 used `type::thing`, renamed in v3).

---

## 1. SurrealDB’s model (what the DB actually has)

- **Record IDs are a native type** (often called `Thing` or “record link” in docs). They are not plain strings.
- In SurrealQL, you create them with **`type::record('table', $key)`** where `$key` is the **raw key** (e.g. `"2025041711441879938520500"`), not `"player:key"`.
- Stored fields like `resulted_in.in`, `resulted_in.out`, or `player.id` are **record ID types** in the DB. Comparing them to strings can fail unless the engine coerces or you cast.
- As of **SurrealDB v2+**, strings are **not** automatically converted to record IDs; you must use **`type::record()`** or an explicit cast (e.g. `<record> string`) when you need a record ID in a query.

So the rule is: **inside SurrealQL and in Rust when talking to the DB, treat IDs as the record-ID type; only at the API boundary do we use a single string format.**

---

## 2. Our canonical string format: `"table/key"` (slash)

**HTTP note:** For `GET/PUT/DELETE` on `/api/games` and `/api/venues`, path parameters are the **raw key** (single segment); JSON still uses `table/key`. See [`api/RESOURCE_IDS_HTTP.md`](api/RESOURCE_IDS_HTTP.md).

Everywhere in **application code and DTOs** (Rust types, JSON APIs, frontend, cache keys):

- Use **one format only:** `"table/key"` with a **slash**, e.g. `"player/2025041711441879938520500"`, `"contest/10860534"`.
- **Do not** use `"table:key"` in DTOs or for map keys / lookups in Rust. Reserve the colon form only for the single case below (INSIDE bindings).

This gives:

- Consistent parsing: strip `"table/"` or `"table:"` to get the raw key for `type::record('table', $key)`.
- No ambiguity: one canonical form for logging, APIs, and cross-service use.

---

## 3. SurrealQL: how we write queries

### 3.1 Single-record lookups (index-friendly)

Always use **`type::record('table', $key)`** and bind the **raw key** (no table prefix):

```sql
WHERE id = type::record('player', $key)
WHERE `in` = type::record('player', $key)
WHERE `out` = type::record('contest', $key)
```

- In Rust: extract the key from the canonical string with **`record_id_to_key(id, "player")`** (or a table-specific helper like `player_id_to_key` that strips `player/`, `player:`, etc.), then bind `("key", key)`.
- This form uses indexes (e.g. `DEFINE INDEX ... ON TABLE resulted_in COLUMNS in`).

### 3.2 Multi-record lookups: `INSIDE $ids`

For **`WHERE id INSIDE $ids`** or **`WHERE out INSIDE $contest_ids`**:

- SurrealDB compares the column (record ID type) to the array. The Rust SDK can send an array of **strings**.
- **Our binding convention:** bind an array of strings in **`"table:key"`** (colon) form, e.g. `["contest:key1", "contest:key2"]`. This matches how the engine often compares record IDs to string representations and works with the current Rust driver.
- So in Rust: when you have a list of canonical IDs in `"table/key"` form, convert to colon form **only for the INSIDE binding**: `ids.iter().map(|s| s.replace('/', ":")).collect::<Vec<_>>()`.

If you ever see INSIDE not matching in a given SurrealDB/engine version, try:

- Building an array of **Thing** in Rust and binding that, or  
- Using a subquery instead of INSIDE, e.g. `WHERE out IN (SELECT VALUE type::record('contest', $k) FROM $keys)` (if your driver supports array iteration).

### 3.3 Edge tables: IN = subject, OUT = object

Relation tables (`played_at`, `played_with`, `resulted_in`) use a single convention so all code and migrations stay consistent:

- **`in`** = **Subject** (the contest): the record that “has” the relationship.
- **`out`** = **Object** (venue, game, or player): the record that the relationship points to.

So:

- **[Contest]–in→(played_at)–out→[Venue]**: contest `in`, venue `out`.
- **[Contest]–in→(played_with)–out→[Game]**: contest `in`, game `out`.
- **[Contest]–in→(resulted_in)–out→[Player]** (with `place`, `result`): contest `in`, player `out`.

In SurrealQL:

- “Contests for this venue” → `SELECT \`in\` FROM played_at WHERE \`out\` = type::record('venue', $key)`.
- “Venue for this contest” → `SELECT \`out\` FROM played_at WHERE \`in\` = type::record('contest', $key)`.

The Arango→Surreal migration maps `_from` → `in`, `_to` → `out`, so subject/object are preserved.

### 3.4 SurrealDB’s standard: Thing vs string

- **Record IDs are a native type** in SurrealDB (Thing/RecordId). The recommended way to compare is **record-to-record**: use `type::record('table', $key)` with a bound **raw key**, or bind a **Thing** from the SDK so the engine compares like types.
- **Strings are not automatically converted to record IDs** (since v2.0). To use a string as a record ID in SurrealQL you must cast (e.g. `<record> string` or the record-id cast per docs) or build a record with `type::record('table', $key)`.
- So the **standard pattern** is: **always use the record type in queries** — either `type::record('table', $key)` with raw key, or a bound Thing. Reserve string comparison for the fallback below.

### 3.5 When Thing comparison fails: string-comparison fallback

In some situations **record-to-record comparison does not match** even when the logical ID is the same (e.g. subquery `(SELECT VALUE id FROM player WHERE ...)[0]` vs stored `resulted_in.out`, or Thing binding from the Rust driver differing from stored representation). When that happens:

- **Resolve the ID in one place**, then use it consistently:
  - **Preferred:** Get the **raw key** (e.g. by `get_player_id_by_email` then `record_id_to_key`) and use **`type::record('table', $key)`** in the main query; or
  - **Fallback:** Get the **exact string form** from the DB (`SELECT string::concat(id) AS id_str FROM table WHERE ...`) and use **string comparison** so both sides are strings and format no longer matters.
- **String-comparison pattern** (when you need the fallback):

  ```sql
  WHERE string::replace(string::concat(`out`), '`', '') = $id_str
  ```

  Bind `id_str` with the value from `string::concat(id)` (or our canonical `"table/key"` normalized to match). Use this only when `type::record('table', $key)` or binding a Thing is unreliable in that code path (e.g. “me” resolved by email, or after hitting driver/engine quirks).

**Rule of thumb:** Prefer Thing/record comparison; if a given query path returns zero rows despite correct data, switch that path to the string-comparison pattern and resolve the id string in one place (e.g. `get_player_id_str_by_email` then one query using `$id_str`).

---

## 4. Rust: repository and response handling

### 4.1 Deserializing query results

- For any column that is a record ID in the schema (`id`, `in`, `out`, `contest_id`, `player_id`, etc.), use **`Option<surrealdb::sql::Thing>`** in your row structs. Do not deserialize record-ID columns as `String` unless you have a custom deserializer that accepts both Thing and string.
- Convert to our canonical string only when building DTOs or keys: use **`thing_to_record_id(&row.id)`** (or the shared helper in `surreal_helpers.rs`) to get `"table/key"`.

### 4.2 Passing IDs into queries

- **Single ID:** you have `"player/abc"` or `"player:abc"` from API/config. Call **`record_id_to_key(id, "player")`** (or `player_id_to_key`) to get `"abc"`, then bind `("key", key)` and use `type::record('player', $key)` in SQL.
- **List of IDs for INSIDE:** you have `Vec<String>` in `"table/key"` form. Convert to colon form for the binding: `ids.iter().map(|s| s.replace('/', ":")).collect::<Vec<_>>()`, then bind e.g. `("contest_ids", contest_ids_colon)`.

### 4.3 Shared helpers (`back/api/src/surreal_helpers.rs`) — use these so all tabs work

**Single-record lookup (one place for UUID/backticks/Thing):**

- **`select_one_by_record_id(db, table, id)`** — async. Fetches one row by canonical `"table/key"` id. Tries UUID lookup first for `contest`/`player`/`game`, then Thing binding. **Use for all “get by id” lookups** so we don’t duplicate translation logic. Allowed tables: `contest`, `player`, `game`, `venue`.

**Reading IDs from query rows:**

- **`record_id_from_row(v, default_table_for_bare_number)`** — extract and normalize record id from a row (checks `id`, `_id`, `player_id`). Handles string, Thing object, or bare number when second arg is `Some("player")`. **Use whenever you read an id from a query row.** Pass `None` for contest/venue/game, `Some("player")` for analytics.
- **`record_id_from_field(v, field_name)`** — same for a single field (e.g. edge `out` or `in`). For INSIDE bindings use `record_ids_to_inside_value()`.

**Thing ↔ canonical string:**

- **`thing_to_record_id(t)`** — from `Option<Thing>`, returns canonical `"table/key"`. **Use for every Thing→string** when building DTOs so the frontend always gets the same format.
- **`record_id_to_key(id, table)`** — from `"table/key"` or `"table:key"`, returns the raw key for bindings. Use when you need a key for `type::record('table', $key)` in SurrealQL (e.g. edge queries).
- **`record_id_to_thing(id, table)`** — builds a Thing for `WHERE id = $rid` when you can’t use `select_one_by_record_id` (e.g. in a custom query).
- **`normalize_record_id_string(s)`** — from a string that might be `"table:key"` or `"table:⟨key⟩"`, returns canonical `"table/key"`.

**INSIDE bindings:**

- **`record_ids_to_inside_value(ids, table)`** — converts `"table/key"` list to `"table:key"` for `INSIDE $ids`.

Repository-specific helpers should call these; avoid new translation logic so we don’t waste time on backticks/UUID/Thing per tab.

---

## 5. Schema and indexes

- Tables and edges that store record links should use the **record ID type** for those fields (SurrealDB’s default when you use `RELATE` or store `type::record(...)`).
- Define indexes on those columns so `WHERE in = type::record(...)` and `WHERE out = type::record(...)` use indexes (see `docs/surreal-indexes-optional.surql`).
- Do **not** store IDs as plain strings in the schema if they represent links to another table; use the native record ID type so comparisons and INSIDE work reliably.

---

## 6. Frontend

- APIs always return canonical `"table/key"` (slash, no backticks) for id fields when the backend uses `record_id_from_row` / `thing_to_record_id`.
- On the frontend, when building URLs or API paths (e.g. `/contest/:id`), normalize whatever the list or API gave you to a **stable key** (e.g. strip `contest/`, `contest:`, backticks) so the detail request uses one format. Use a single helper (e.g. `contest_key_from_any` in `api/contests.rs` or a generic `record_key_from_any(id, "contest")`) so all pages use the same rule.

---

## 7. Summary table

| Context | Use | Example |
|--------|-----|--------|
| **API / DTOs / app code** | String, canonical form | `"player/2025041711441879938520500"` |
| **SurrealQL single lookup** | `type::record('table', $key)` | Bind raw key `"2025041711441879938520500"` |
| **SurrealQL when Thing fails** | String comparison fallback | Resolve id string (e.g. `get_player_id_str_by_email`); compare with `string::replace(string::concat(out), …) = $id_str` (see §3.5) |
| **SurrealQL INSIDE** | `INSIDE $ids` | Bind `Vec<String>` in `"table:key"` form |
| **Rust row struct** | `Option<surrealdb::sql::Thing>` | For `id`, `in`, `out`, etc. |
| **Rust: row → DTO** | `thing_to_record_id(&row.id)` from `surreal_helpers` | → `"table/key"` (backticks stripped) |
| **Rust: API string → query** | `record_id_to_key(id, "player")` | → raw key for `type::record` |
| **Rust: get one row by id** | `select_one_by_record_id(db, "contest", id)` | → one place for UUID/Thing |

---

## 8. Migration from ArangoDB

- ArangoDB used `_id` strings (e.g. `"players/123"`). We already use **slash** in our canonical form, which matches that habit.
- In SurrealDB we **never** rely on “string happens to match record ID”; we use **`type::record('table', $key)`** in queries and record-id type in Rust for ID columns. The only place we intentionally use string form for the DB is the **INSIDE** binding (`"table:key"`), as above.
- When converting ArangoDB data or tools (e.g. `arango-to-surreal`), ensure:
  - Stored record links are the **record ID type**, not strings.
  - Application code and APIs then use the single **`"table/key"`** convention and the helpers above so the whole stack stays consistent and efficient.

This gives a single, SurrealDB-recommended and industry-standard approach: **Thing in the DB and in query logic, one string format at the edges, and explicit conversion only at the boundaries.**
