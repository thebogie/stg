# Migration Testing Workflow

This document explains how to test migrations properly using the **wipe-and-migrate** approach, then deploy to production with existing data.

## The Problem

Migrations need to work in two scenarios:
1. **Fresh install**: Empty database → Run migrations → Database created
2. **Existing data**: Production backup → Run migrations → Data updated

## The Solution: Two-Phase Testing

### Phase 1: Test Migrations from Scratch (Local)

Test that migrations can create the database structure from nothing:

```bash
./scripts/test-migrations-workflow.sh
```

**What it does:**
1. Builds production containers
2. Loads production data (to have realistic data structure)
3. **Wipes the database** (drops `smacktalk` database)
4. Runs migrations from scratch
5. Verifies database structure is created correctly

**Why wipe?**
- Tests that migrations work on a clean slate
- Ensures migrations can create everything needed
- Catches issues with migration ordering
- Validates that migrations are complete

### Phase 2: Test Migrations on Existing Data (Local)

Test that migrations work correctly when applied to existing production data:

```bash
./scripts/test-migrations-on-existing-data.sh [backup-file]
```

**What it does:**
1. Restores production backup
2. Clears migration state (so migrations run again)
3. Runs migrations on existing data
4. Verifies data integrity maintained

**Why test on existing data?**
- Ensures migrations are idempotent
- Tests data transformation logic
- Validates that existing data isn't corrupted
- Catches issues with data migrations

### Phase 3: Deploy to Production

Deploy tested images and run migrations on production data:

```bash
# On production server
./scripts/deploy-with-migrations.sh --version v<version>
```

**What it does:**
1. Backs up current production database
2. Deploys tested images
3. **Data is already in volumes** (no restore needed normally)
4. Runs migrations on existing production data
5. Verifies deployment

## Complete Workflow

### Step 1: Build and Test Migrations from Scratch

```bash
# Build production images
./scripts/build-prod-images.sh

# Test migrations from scratch (wipe → migrate)
./scripts/test-migrations-workflow.sh
```

This ensures:
- ✅ Migrations can create database from nothing
- ✅ All collections and indexes are created
- ✅ Migration ordering is correct
- ✅ No missing dependencies

### Step 2: Test Migrations on Existing Data

```bash
# Get production backup (or use existing)
./scripts/backup-prod-db.sh --local  # If you have prod data locally
# OR transfer from production:
# scp production:/backups/arangodb/latest-backup.tar.gz ./_build/backups/

# Test migrations on existing data
./scripts/test-migrations-on-existing-data.sh ./_build/backups/latest-backup.tar.gz
```

This ensures:
- ✅ Migrations work with existing data
- ✅ Data transformations are correct
- ✅ No data loss or corruption
- ✅ Migrations are idempotent

### Step 3: Export Tested Images

```bash
./scripts/export-tested-images.sh
```

### Step 4: Deploy to Production

```bash
# On production server

# 1. Transfer images
scp _build/artifacts/*.tar.gz* user@production:/tmp/

# 2. Deploy with migrations
./scripts/deploy-with-migrations.sh --version v<version> --image-dir /tmp
```

**What happens in production:**
1. Backup is created automatically
2. Tested images are deployed
3. **Production data is already in volumes** (no restore needed)
4. Migrations run on existing production data
5. Application starts with migrated database

## How Migrations Work

### Migration System Features

Your migration system tracks:
- **Applied migrations**: Stored in `_schema_migrations` collection
- **Migration state**: Prevents re-running already-applied migrations
- **Checksums**: Validates migration files haven't changed
- **Locking**: Prevents concurrent migration runs

### Migration Idempotency

Migrations should be designed to be **idempotent**:

**Good (Idempotent):**
```aql
// Only update if field doesn't exist
FOR u IN users
  FILTER !HAS(u, "status")
  UPDATE u WITH { status: "active" } IN users
```

**Bad (Not Idempotent):**
```aql
// This will fail if run twice
FOR u IN users
  UPDATE u WITH { status: "active" } IN users
```

### Schema Migrations

Schema migrations (`.json` files) are automatically idempotent:
- `create_collection`: Only creates if doesn't exist
- `ensure_index`: Only creates if doesn't exist

### Data Migrations

Data migrations (`.aql` files) need to be written idempotently:
- Check if change is needed before applying
- Use `FILTER` to only update what needs updating
- Avoid unconditional updates

## Why This Approach Works

### 1. Tests Both Scenarios

- **Fresh install**: Wipe → Migrate (tests migration completeness)
- **Existing data**: Restore → Migrate (tests migration safety)

### 2. Production Data Safety

- Production data stays in volumes
- No need to restore (data is already there)
- Migrations run on existing data
- Backup created before migrations

### 3. Migration Confidence

- Tested from scratch (ensures completeness)
- Tested on existing data (ensures safety)
- Same migrations run in production
- No surprises

## Example: Adding a New Field

Let's say you want to add a `status` field to all users:

### 1. Create Migration

```aql
// migrations/files/20250120T000000_add_status_to_users.aql
FOR u IN users
  FILTER !HAS(u, "status")
  UPDATE u WITH { status: "active" } IN users
```

### 2. Test from Scratch

```bash
./scripts/test-migrations-workflow.sh
```

This creates database from scratch, runs all migrations including the new one.

### 3. Test on Existing Data

```bash
# Restore production backup
./scripts/test-migrations-on-existing-data.sh ./_build/backups/prod-backup.tar.gz
```

This restores production data, then runs migrations (including the new one) on existing users.

### 4. Deploy

```bash
# On production
./scripts/deploy-with-migrations.sh --version v<version>
```

Migrations run on production data, adding `status` field to all existing users.

## Troubleshooting

### Migration Fails on Fresh Install

**Problem**: Migration fails when running from scratch

**Solution**: 
- Check migration dependencies
- Ensure all required collections exist
- Test with `./scripts/test-migrations-workflow.sh`

### Migration Fails on Existing Data

**Problem**: Migration fails when applied to existing data

**Solution**:
- Check for data conflicts
- Ensure migration is idempotent
- Test with `./scripts/test-migrations-on-existing-data.sh`

### Migration Already Applied

**Problem**: Migration system says migration already applied

**Solution**:
- Check `_schema_migrations` collection
- Clear migration state if needed (for testing)
- In production, migrations automatically skip already-applied ones

### Data Corruption After Migration

**Problem**: Data is corrupted after migration

**Solution**:
- Restore from backup (created automatically)
- Fix migration script
- Re-test before deploying again

## Best Practices

1. **Always test both scenarios**
   - Fresh install (wipe → migrate)
   - Existing data (restore → migrate)

2. **Write idempotent migrations**
   - Check before updating
   - Use filters to avoid duplicate work

3. **Backup before migrations**
   - Automatic in production
   - Manual for testing

4. **Test with production data**
   - Use real production backups
   - Test data transformations

5. **Verify after migrations**
   - Check data integrity
   - Verify schema changes
   - Test application functionality

## Summary

**Local Testing:**
1. Wipe database → Run migrations (tests completeness)
2. Restore backup → Run migrations (tests safety)

**Production Deployment:**
1. Backup database
2. Deploy tested images
3. Data already in volumes (no restore)
4. Run migrations on existing data
5. Verify deployment

This ensures migrations work correctly in both scenarios and production data is always safe!

