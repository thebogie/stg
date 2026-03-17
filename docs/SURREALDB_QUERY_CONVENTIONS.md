# SurrealDB query conventions (backend)

**SurrealDB v3.** See **docs/SURREALDB_ID_CONVENTIONS.md** for ID formats.

## RecordId binding for read paths (recommended)

For **read paths** (SELECT, WHERE by id or edge endpoint), prefer **binding the SDK’s `RecordId`** instead of building IDs in SurrealQL with `type::record('table', $key)`:

- **Pattern:** `let record_id = surrealdb::types::RecordId::new("table", key.as_str());` then `WHERE id = $record_id` (or `WHERE \`out\` = $record_id`, etc.) and `.bind(("record_id", record_id))`.
- **Why:** SurrealDB’s native record ID type is sent and matched consistently; binding `RecordId` avoids string/key serialization mismatches and is the type-safe, SurrealDB-recommended approach for lookups. Writes (CREATE, INSERT, UPDATE) may still use `type::record('table', $key)` where that fits the API.

## One statement per query (no multi-statement)

Use **one SurrealQL statement per `.query()` call**. Do not send multiple statements in one string (e.g. `SELECT ...; SELECT ...`).

- **Why:** The Rust client returns one result set per statement via `.take(0)`, `.take(1)`, etc. Multi-statement queries require multiple `take(n)` calls and can behave inconsistently (e.g. scalar subqueries in the first statement sometimes returning wrong or unexpected shapes). Single-statement calls give one result set per round-trip and avoid that.
- **Efficiency:** SurrealDB’s docs don’t mandate single vs multi-statement for “best practice”; batching multiple operations in one request can reduce round-trips when using the `( LET ...; UPDATE ...; RETURN ... )` style. For our backend, we standardize on one statement per `.query()` so each call has exactly one result set and behavior is predictable. If you need two result sets (e.g. games and venues), run two separate `.query()` calls.
- **ArangoDB legacy:** ArangoDB AQL often used multi-query or chained logic; the SurrealDB port uses single-statement queries only.

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
