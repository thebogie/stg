# STG_RD ArangoDB Migration Tool

A Rust-based tool for managing ArangoDB schema and data migrations in the STG_RD project.

example for dev arangodb:  
clear; cargo build --package stg-rd-migrations --release;  ./scripts/clear-lock.sh --password <pass> ;  ./run-migrations.sh --password <pass> --database smacktalk --username root --endpoint http://localhost:<port>

## Features

- **Schema migrations**: Create collections, indexes, and other database objects
- **Data migrations**: Run AQL queries for data backfills and updates
- **Version tracking**: Automatically tracks applied migrations in `_schema_migrations` collection
- **Locking**: Prevents concurrent migration runs
- **Dry-run mode**: Preview changes without applying them
- **Checksums**: Validates migration file integrity

## Migration File Types

### JSON Schema Migrations (`.json`)

Define collections, indexes, and other schema objects:

```json
{
  "steps": [
    {
      "type": "create_collection",
      "name": "users",
      "collection_type": "document",
      "options": {
        "replicationFactor": 2,
        "writeConcern": 1
      }
    },
    {
      "type": "ensure_index",
      "collection": "users",
      "index": {
        "type": "persistent",
        "fields": ["email"],
        "unique": true,
        "sparse": true
      }
    }
  ]
}
```

### AQL Data Migrations (`.aql`)

Run AQL queries for data updates:

```aql
FOR u IN users
  FILTER !HAS(u, "status")
  UPDATE u WITH { status: "active" } IN users
```

## Usage

### Build

```bash
# From workspace root
cargo build --package stg-rd-migrations

# Or from migrations directory
cargo build --release
```

### Run Migrations

```bash
# Basic usage
./target/release/stg-rd-migrations \
  --endpoint http://127.0.0.1:8529 \
  --database stg_rd \
  --username root \
  --password "$ARANGO_PASSWORD" \
  --migrations-dir ./files

# With environment variables
export ARANGO_ENDPOINT="http://127.0.0.1:8529"
export ARANGO_DATABASE="stg_rd"
export ARANGO_USERNAME="root"
export ARANGO_PASSWORD="your_password"
export MIGRATIONS_DIR="./files"

./target/release/stg-rd-migrations

# Dry-run to preview changes
./target/release/stg-rd-migrations --dry-run
```

### Environment Variables

- `ARANGO_ENDPOINT`: ArangoDB server endpoint (e.g., `http://127.0.0.1:8529`)
- `ARANGO_DATABASE`: Database name to run migrations against
- `ARANGO_USERNAME`: Username for authentication
- `ARANGO_PASSWORD`: Password for authentication
- `MIGRATIONS_DIR`: Directory containing migration files

## Migration Naming Convention

Use timestamped filenames to ensure proper ordering:

- `20250820T120000_bootstrap.json` - Bootstrap collections and indexes
- `20250820T123000_add_user_status.aql` - Data migration
- `20250820T124000_add_tournament_metadata.json` - Schema updates

## How It Works

1. **Authentication**: Connects to ArangoDB using JWT authentication
2. **Locking**: Acquires a lock in `_migration_lock` collection to prevent concurrent runs
3. **Tracking**: Checks `_schema_migrations` collection for already-applied migrations
4. **Execution**: Runs new migrations in filename order
5. **Recording**: Records each applied migration with checksum and timing
6. **Cleanup**: Releases lock when complete

## Migration Steps

### Create Collection
```json
{
  "type": "create_collection",
  "name": "collection_name",
  "collection_type": "document", // or "edge"
  "options": {
    "replicationFactor": 2,
    "writeConcern": 1
  }
}
```

### Ensure Index
```json
{
  "type": "ensure_index",
  "collection": "collection_name",
  "index": {
    "type": "persistent",
    "fields": ["field1", "field2"],
    "unique": true,
    "sparse": true
  }
}
```

### Run AQL
```json
{
  "type": "aql",
  "query": "FOR doc IN collection UPDATE doc WITH { field: 'value' } IN collection",
  "bind_vars": {
    "param": "value"
  }
}
```

## Best Practices

1. **Ordering**: Use timestamped filenames to ensure proper execution order
2. **Idempotency**: Make migrations safe to re-run when possible
3. **Small Changes**: Keep individual migrations focused and small
4. **Testing**: Test migrations on staging before production
5. **Backups**: Take database backups before destructive migrations
6. **Dry Runs**: Always use `--dry-run` first to preview changes

## Troubleshooting

### Lock Already Held
If you get a "Lock already held" error:
1. Check if another migration process is running
2. If not, manually delete the lock: `DELETE FROM _migration_lock WHERE _key = 'lock'`

### Migration Failed
1. Check the error message for details
2. Fix the migration file
3. Re-run the migration tool (it will skip already-applied migrations)

### Checksum Mismatch
If a migration file has been modified after being applied:
1. The tool will detect the checksum change
2. You may need to manually handle the discrepancy
3. Consider creating a new migration instead of modifying existing ones

## Integration with CI/CD

Add to your deployment pipeline:

```bash
# Run migrations before deploying application
cargo run --package stg-rd-migrations \
  --endpoint "$ARANGO_ENDPOINT" \
  --database "$ARANGO_DATABASE" \
  --username "$ARANGO_USERNAME" \
  --password "$ARANGO_PASSWORD" \
  --migrations-dir "./files"
```

## Security Notes

- Store database credentials securely (use environment variables)
- Use read-only users for dry-run testing when possible
- Consider using ArangoDB's built-in user management for production
- Rotate passwords regularly
