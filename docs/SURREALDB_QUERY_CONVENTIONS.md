# SurrealDB query conventions (backend)

## Record IDs and `take(0)` into JSON Value

When you run a query and deserialize the result with `.take(0)` into `Vec<serde_json::Value>` (or `Vec<Value>`), the SurrealDB Rust client expects **JSON-compatible** values. SurrealDB returns record IDs (`id`, or edge endpoints like `in`/`out`) as **Thing/RecordId** types, which the client serializes as Rust enums. That causes:

```text
Serialization error: invalid type: enum, expected any valid JSON value
```

**Fix:** In the SELECT, turn record IDs into strings so the response is JSON-serializable:

- **Table record id:**  
  `SELECT string::concat(id) AS id, ... FROM player`
- **Edge endpoint:**  
  `SELECT string::concat(pw.\`in\`) AS game_id FROM ...`
- **Aliased id:**  
  `SELECT string::concat(id) AS contest_id, ... FROM contest`

Use the same pattern for any column that is a record reference when the result is taken as `serde_json::Value`.

## When you don’t need to change the query

- **Typed structs:** If you deserialize into a struct that has `id: Option<surrealdb::sql::Thing>`, the client can deserialize the Thing directly; no `string::concat` needed.
- **No record IDs in SELECT:** Queries that only return scalars (e.g. `count()`, `name`, `start`) are fine as-is.

## Where this was applied

- **Leaderboard:** `SELECT string::concat(id) AS id, handle, firstname, email FROM player`
- **Client analytics:** player list by IDs, get player id by email
- **Analytics:** contest stats, recent contests, player achievements (player id, game_id, venue_id)
- **Contest repository:** list contest ids, list contests (id, name, start, stop)

Adding new queries that return rows as `Vec<serde_json::Value>` should follow the same convention.
