# Contest Creator Tracking Migration

This directory contains AQL migration scripts to add creator tracking fields to existing contests in the database.

## Migration Overview

The migration adds two new fields to all existing contests:
- `creator_id`: Set to `"player/2025041711441879994340500"` (default creator)
- `created_at`: Set to the contest's `stop` time

## Migration Scripts

### 1. `complete_contest_creator_migration.aql` (Recommended)
**Use this script for a complete migration with verification.**

This script:
- Shows pre-migration state
- Performs the migration
- Verifies the results
- Provides detailed success/failure reporting

### 2. `migrate_contest_creator_tracking.aql` (Simple)
**Use this script for a quick migration without verification.**

This script simply updates all contests with the new fields.

### 3. `verify_contest_creator_migration.aql` (Verification Only)
**Use this script to check migration status without making changes.**

This script shows the current state of creator tracking fields.

### 4. `add_creator_tracking_to_existing_contests.aql` (Analysis Only)
**Use this script to analyze current state before migration.**

This script shows what contests exist and their current structure (commented out migration code).

## How to Run the Migration

### Option 1: Complete Migration (Recommended)
```bash
# Run the complete migration with verification
arangosh --server.database stg_rd --javascript.execute-string "
var fs = require('fs');
var query = fs.readFileSync('complete_contest_creator_migration.aql', 'utf8');
db._query(query).toArray();
"
```

### Option 2: Simple Migration
```bash
# Run the simple migration
arangosh --server.database stg_rd --javascript.execute-string "
var fs = require('fs');
var query = fs.readFileSync('migrate_contest_creator_tracking.aql', 'utf8');
db._query(query).toArray();
"
```

### Option 3: Using ArangoDB Web Interface
1. Open ArangoDB web interface
2. Go to the `stg_rd` database
3. Navigate to "Queries" tab
4. Copy and paste the migration script
5. Click "Execute"

## Expected Results

After running the migration, you should see:
- All contests have a `creator_id` field set to `"player/2025041711441879994340500"`
- All contests have a `created_at` field set to their `stop` time
- The migration report shows 100% success rate

## Verification

After migration, run the verification script to confirm:
```bash
arangosh --server.database stg_rd --javascript.execute-string "
var fs = require('fs');
var query = fs.readFileSync('verify_contest_creator_migration.aql', 'utf8');
db._query(query).toArray();
"
```

## Rollback (If Needed)

If you need to rollback the migration, you can remove the new fields:
```aql
FOR contest IN contest
    UPDATE contest WITH {
        creator_id: null,
        created_at: null
    } IN contest
    RETURN OLD._id
```

## Notes

- The migration is **idempotent** - you can run it multiple times safely
- The `creator_id` uses the provided default value: `"player/2025041711441879994340500"`
- The `created_at` field is set to the contest's `stop` time as requested
- All existing contests will be updated, regardless of their current state

