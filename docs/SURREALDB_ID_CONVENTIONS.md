# SurrealDB ID Conventions — Project Standard

This document defines how we handle **record IDs** (Thing vs string) across the codebase so we avoid the mixing that causes subtle bugs. It aligns with SurrealDB’s model and gives one consistent pattern for queries, bindings, and APIs.

---

## 1. SurrealDB’s model (what the DB actually has)

- **Record IDs are a native type** (often called `Thing` or “record link” in docs). They are not plain strings.
- In SurrealQL, you create them with **`type::thing('table', $key)`** where `$key` is the **raw key** (e.g. `"2025041711441879938520500"`), not `"player:key"`.
- Stored fields like `resulted_in.in`, `resulted_in.out`, or `player.id` are **record ID types** in the DB. Comparing them to strings can fail unless the engine coerces or you cast.
- As of **SurrealDB v2**, strings are **not** automatically converted to record IDs; you must use **`type::thing()`** or an explicit cast (e.g. `<record> string`) when you need a record ID in a query.

So the rule is: **inside SurrealQL and in Rust when talking to the DB, treat IDs as the record-ID type; only at the API boundary do we use a single string format.**

---

## 2. Our canonical string format: `"table/key"` (slash)

Everywhere in **application code and DTOs** (Rust types, JSON APIs, frontend, cache keys):

- Use **one format only:** `"table/key"` with a **slash**, e.g. `"player/2025041711441879938520500"`, `"contest/10860534"`.
- **Do not** use `"table:key"` in DTOs or for map keys / lookups in Rust. Reserve the colon form only for the single case below (INSIDE bindings).

This gives:

- Consistent parsing: strip `"table/"` or `"table:"` to get the raw key for `type::thing('table', $key)`.
- No ambiguity: one canonical form for logging, APIs, and cross-service use.

---

## 3. SurrealQL: how we write queries

### 3.1 Single-record lookups (index-friendly)

Always use **`type::thing('table', $key)`** and bind the **raw key** (no table prefix):

```sql
WHERE id = type::thing('player', $key)
WHERE `in` = type::thing('player', $key)
WHERE `out` = type::thing('contest', $key)
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
- Using a subquery instead of INSIDE, e.g. `WHERE out IN (SELECT VALUE type::thing('contest', $k) FROM $keys)` (if your driver supports array iteration).

---

## 4. Rust: repository and response handling

### 4.1 Deserializing query results

- For any column that is a record ID in the schema (`id`, `in`, `out`, `contest_id`, `player_id`, etc.), use **`Option<surrealdb::sql::Thing>`** in your row structs. Do not deserialize record-ID columns as `String` unless you have a custom deserializer that accepts both Thing and string.
- Convert to our canonical string only when building DTOs or keys: use **`thing_to_record_id(&row.id)`** (or the shared helper in `surreal_helpers.rs`) to get `"table/key"`.

### 4.2 Passing IDs into queries

- **Single ID:** you have `"player/abc"` or `"player:abc"` from API/config. Call **`record_id_to_key(id, "player")`** (or `player_id_to_key`) to get `"abc"`, then bind `("key", key)` and use `type::thing('player', $key)` in SQL.
- **List of IDs for INSIDE:** you have `Vec<String>` in `"table/key"` form. Convert to colon form for the binding: `ids.iter().map(|s| s.replace('/', ":")).collect::<Vec<_>>()`, then bind e.g. `("contest_ids", contest_ids_colon)`.

### 4.3 Shared helpers (`back/api/src/surreal_helpers.rs`)

- **`record_id_to_key(id, table)`** — from any `"table/key"` or `"table:key"` string, returns the raw key for `type::thing(table, $key)` (strips backticks and angle brackets).
- **`thing_to_record_id(t)`** — from `Option<Thing>`, returns canonical `"table/key"` with **backticks stripped**. SurrealDB may serialize keys with backticks (e.g. `player/\`uuid\``); this helper ensures the same canonical form everywhere so comparisons (e.g. “is this row the current player?”) and API responses are consistent. **Use this for every Thing→string conversion** in the backend so frontend and all repos see one format.

Repository-specific helpers (e.g. `player_id_to_key` in analytics) should normalize input to the **raw key** for binding; for **output** (DB row → string) always use **`thing_to_record_id`** from `surreal_helpers.rs`. Prefer reusing `record_id_to_key` where possible.

---

## 5. Schema and indexes

- Tables and edges that store record links should use the **record ID type** for those fields (SurrealDB’s default when you use `RELATE` or store `type::thing(...)`).
- Define indexes on those columns so `WHERE in = type::thing(...)` and `WHERE out = type::thing(...)` use indexes (see `docs/surreal-indexes-optional.surql`).
- Do **not** store IDs as plain strings in the schema if they represent links to another table; use the native record ID type so comparisons and INSIDE work reliably.

---

## 6. Summary table

| Context | Use | Example |
|--------|-----|--------|
| **API / DTOs / app code** | String, canonical form | `"player/2025041711441879938520500"` |
| **SurrealQL single lookup** | `type::thing('table', $key)` | Bind raw key `"2025041711441879938520500"` |
| **SurrealQL INSIDE** | `INSIDE $ids` | Bind `Vec<String>` in `"table:key"` form |
| **Rust row struct** | `Option<surrealdb::sql::Thing>` | For `id`, `in`, `out`, etc. |
| **Rust: row → DTO** | `thing_to_record_id(&row.id)` from `surreal_helpers` | → `"table/key"` (backticks stripped) |
| **Rust: API string → query** | `record_id_to_key(id, "player")` | → raw key for `type::thing` |

---

## 7. Migration from ArangoDB

- ArangoDB used `_id` strings (e.g. `"players/123"`). We already use **slash** in our canonical form, which matches that habit.
- In SurrealDB we **never** rely on “string happens to match record ID”; we use **`type::thing('table', $key)`** in queries and **Thing** in Rust for ID columns. The only place we intentionally use string form for the DB is the **INSIDE** binding (`"table:key"`), as above.
- When converting ArangoDB data or tools (e.g. `arango-to-surreal`), ensure:
  - Stored record links are the **record ID type**, not strings.
  - Application code and APIs then use the single **`"table/key"`** convention and the helpers above so the whole stack stays consistent and efficient.

This gives a single, SurrealDB-recommended and industry-standard approach: **Thing in the DB and in query logic, one string format at the edges, and explicit conversion only at the boundaries.**
