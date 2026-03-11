//! Shared SurrealDB client type for the backend.
//! Use this type everywhere instead of the concrete engine type.
//!
//! **Query convention:** When deserializing into `Vec<serde_json::Value>` (or plain `Value`),
//! record IDs must be selected as strings or deserialization can fail with "invalid type: enum".
//! Use `string::concat(id) AS id` (or equivalent for other record refs) in SELECT. See
//! `docs/SURREALDB_QUERY_CONVENTIONS.md`.

use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

/// SurrealDB client (WebSocket). Use for all repository and health code.
pub type Db = Surreal<Client>;
