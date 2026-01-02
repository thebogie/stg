# Test-Then-Deploy Workflow Guide

This guide explains how to test production Docker containers locally and deploy the exact tested artifacts to production, eliminating the need for `git pull` in production.

## Overview

**Problem**: Currently, you do `git pull` in production, which means:
- Untested code runs in production
- Build happens on production server (slow, messy)
- No guarantee that what you tested is what gets deployed

**Solution**: Build production images locally → Test them → Deploy exact tested images to production

## Key Concepts

### 1. Containers vs Data
- **Containers** = Your code (frontend/backend images)
- **Volumes** = Your data (ArangoDB, Redis data on the server)
- When you deploy new containers, they attach to existing volumes
- **Data stays on the server, containers are replaced**

### 2. Image Versioning
- Images are tagged with git commit hash + timestamp
- Example: `vabc123-20240115-143022`
- This ensures you deploy exactly what you tested

### 3. Migration Safety
- Always test migrations with production data snapshots first
- Backup before running migrations in production
- Migrations run after containers are deployed

## Complete Workflow

### Phase 1: Build and Test Locally

#### Step 1: Build Production Images

```bash
./scripts/build-prod-images.sh
```

**What it does:**
- Builds production Docker images (frontend + backend)
- Tags images with version (git commit + timestamp)
- Creates `_build/.build-version` file with version info

**Output:**
- Images: `stg_rd-frontend:v<commit>-<timestamp>`
- Version file: `_build/.build-version`

#### Step 2: Get Production Data Snapshot

**Option A: Export from Production (Recommended)**

```bash
# On production server or via script
cargo run --package scripts --bin export_prod_data \
  -- --arango-url http://production-server:50011 \
  --arango-password <password> \
  --output _build/backups/prod-snapshot-$(date +%Y%m%d).json

# Transfer to dev machine
scp production-server:/path/to/prod-snapshot.json ./_build/backups/
```

**Option B: Use Existing Backup**

```bash
# Place dump file in _build/backups/ directory
cp /path/to/prod-dump.zip _build/backups/smacktalk.zip
```

#### Step 3: Test Production Containers

**⚠️ Important: Data Isolation**

Testing will modify your database! Use one of these approaches:

**Option A: Use Test Database (Recommended)**
```bash
./scripts/test-prod-containers.sh --use-test-db
```
- Uses a separate database (`smacktalk_test` instead of `smacktalk`)
- Production data stays untouched
- Tests can modify test database freely

**Option B: Create Snapshot Before Testing**
```bash
# Create snapshot before testing
./scripts/create-test-snapshot.sh

# Test (this will modify data)
./scripts/test-prod-containers.sh

# Restore snapshot after testing
./scripts/restore-test-snapshot.sh
```

**Option C: Automatic Backup/Restore**
```bash
./scripts/test-prod-containers.sh --backup-before --restore-after
```
- Creates backup before testing
- Prompts to restore after you're done

**What it does:**
- Starts production containers locally
- Loads production data (if available)
- Runs migrations (if any)
- Waits for services to be healthy

**Options:**
- `--skip-data-load`: Skip loading production data
- `--run-tests`: Run automated test suite
- `--use-test-db`: Use separate test database (prevents contamination)
- `--backup-before`: Create backup before testing
- `--restore-after`: Restore from backup after testing

**Manual Testing:**
- Backend: http://localhost:50012
- Frontend: http://localhost:50013
- ArangoDB: http://localhost:50011

#### Step 4: Run Your Tests

```bash
# Run automated tests
./scripts/test-prod-containers.sh --run-tests

# Or manually test:
# - API endpoints
# - Frontend functionality
# - Integration tests
# - Performance tests
```

#### Step 5: Export Tested Images

Once tests pass:

```bash
./scripts/export-tested-images.sh
```

**What it does:**
- Exports tested images to tar.gz files
- Creates checksums for verification
- Saves deployment instructions

**Output:**
- `_build/artifacts/frontend-v<version>.tar.gz`
- `_build/artifacts/backend-v<version>.tar.gz`
- `_build/artifacts/deploy-info-<version>.txt`
- Checksum files

### Phase 2: Deploy to Production

#### Step 1: Transfer Images to Production

**Option A: Direct File Transfer**

```bash
# From development machine
scp _build/artifacts/*.tar.gz* user@production-server:/tmp/
```

**Option B: Docker Registry (Recommended)**

```bash
# Tag and push to registry
docker tag stg_rd-frontend:v<version> your-registry/stg_rd-frontend:v<version>
docker tag stg_rd-backend:v<version> your-registry/stg_rd-backend:v<version>
docker push your-registry/stg_rd-frontend:v<version>
docker push your-registry/stg_rd-backend:v<version>

# On production, pull from registry
docker pull your-registry/stg_rd-frontend:v<version>
docker pull your-registry/stg_rd-backend:v<version>
```

#### Step 2: Backup Production Database

**On production server:**

```bash
./scripts/backup-prod-db.sh
```

**What it does:**
- Creates backup of ArangoDB before deployment
- Saves to `/backups/arangodb/` (or specified directory)
- Creates compressed tar.gz file

**Important**: Always backup before migrations!

#### Step 3: Deploy Tested Images

**On production server:**

```bash
./scripts/deploy-tested-images.sh --version v<version> --image-dir /tmp
```

**What it does:**
- Loads tested images from tar.gz files
- Verifies checksums
- Stops old containers
- Starts new containers with tested images
- Runs migrations (if any)
- Verifies deployment

**Options:**
- `--version TAG`: Specify version tag
- `--image-dir DIR`: Directory with image files (default: /tmp)
- `--skip-backup`: Skip database backup (not recommended)
- `--skip-migrations`: Skip running migrations

#### Step 4: Verify Deployment

```bash
# Check container status
docker compose --env-file config/.env.production \
  -f deploy/docker-compose.yaml \
  -f deploy/docker-compose.prod.yml ps

# Check logs
docker compose --env-file config/.env.production \
  -f deploy/docker-compose.yaml \
  -f deploy/docker-compose.prod.yml logs -f

# Test endpoints
curl http://localhost:50012/health
curl http://localhost:50013
```

## Quick Reference

### Development Workflow

```bash
# 1. Build production images
./scripts/build-prod-images.sh

# 2. Test with production data
./scripts/test-prod-containers.sh

# 3. Export tested images
./scripts/export-tested-images.sh
```

### Production Deployment

```bash
# 1. Transfer images (from dev machine)
scp _build/artifacts/*.tar.gz* user@production:/tmp/

# 2. On production server: Backup database
./scripts/backup-prod-db.sh

# 3. Deploy tested images
./scripts/deploy-tested-images.sh --version v<version> --image-dir /tmp
```

## Handling Database Migrations

### The Migration Testing Strategy

Migrations need to work in two scenarios:
1. **Fresh install**: Empty database → Run migrations
2. **Existing data**: Production data → Run migrations

### Testing Migrations: Two-Phase Approach

#### Phase 1: Test from Scratch (Wipe → Migrate)

```bash
./scripts/test-migrations-workflow.sh
```

**What it does:**
- Loads production data (to have realistic structure)
- **Wipes the database** (drops `smacktalk`)
- Runs migrations from scratch
- Verifies database structure

**Why wipe?**
- Tests that migrations can create everything from nothing
- Ensures migration completeness
- Catches ordering issues

#### Phase 2: Test on Existing Data (Restore → Migrate)

```bash
./scripts/test-migrations-on-existing-data.sh [backup-file]
```

**What it does:**
- Restores production backup
- Clears migration state
- Runs migrations on existing data
- Verifies data integrity

**Why test on existing data?**
- Ensures migrations work with real data
- Tests data transformations
- Validates idempotency

### Production Deployment with Migrations

```bash
# On production server
./scripts/deploy-with-migrations.sh --version v<version>
```

**What happens:**
1. Backs up current production database
2. Deploys tested images
3. **Production data is already in volumes** (no restore needed)
4. Runs migrations on existing production data
5. Verifies deployment

**Key point**: In production, data persists in volumes. You don't need to restore - migrations run on the existing data.

### Migration Best Practices

- ✅ Test from scratch (wipe → migrate)
- ✅ Test on existing data (restore → migrate)
- ✅ Always backup before migrations in production
- ✅ Write idempotent migrations
- ✅ Use dry-run mode first
- ✅ Keep migrations small and focused

See [MIGRATION_TESTING_WORKFLOW.md](MIGRATION_TESTING_WORKFLOW.md) for complete details.

## Data Isolation and Testing

### The Problem

When you test production containers with production data, your tests will **modify the database**. This means:
- Data gets contaminated for future tests
- You might accidentally use modified data
- Hard to get back to a clean state

### Solutions

#### 1. Use Test Database (Best for Regular Testing)

```bash
./scripts/test-prod-containers.sh --use-test-db
```

**How it works:**
- Uses `smacktalk_test` database instead of `smacktalk`
- Production data in `smacktalk` stays untouched
- Tests modify only the test database

**Pros:**
- Simple, no manual steps
- Production data always safe
- Can run tests multiple times

**Cons:**
- Need to load production data into test database first
- Slightly more setup

#### 2. Create Snapshots (Best for One-Time Testing)

```bash
# Before testing
./scripts/create-test-snapshot.sh

# Test (modifies data)
./scripts/test-prod-containers.sh

# After testing, restore
./scripts/restore-test-snapshot.sh
```

**How it works:**
- Creates a backup snapshot before testing
- You test and modify data
- Restore snapshot to get back to clean state

**Pros:**
- Works with actual production database
- Can test exact production setup
- Easy to restore

**Cons:**
- Manual steps required
- Need to remember to restore

#### 3. Automatic Backup/Restore

```bash
./scripts/test-prod-containers.sh --backup-before --restore-after
```

**How it works:**
- Automatically creates backup before testing
- Prompts you when done to restore
- One command handles everything

**Pros:**
- Automated
- Safe by default

**Cons:**
- Still modifies production database temporarily
- Need to confirm restore

### Recommendation

**For regular development/testing:**
- Use `--use-test-db` option
- Load production data into test database once
- Run tests freely without worry

**For pre-deployment validation:**
- Use snapshot approach
- Test with exact production database
- Restore after validation

## Troubleshooting

### Images Not Found

```bash
# Check available images
docker images | grep stg_rd

# Rebuild if needed
./scripts/build-prod-images.sh
```

### Data Not Loading

```bash
# Check if dump file exists
ls -lh _build/backups/

# Check ArangoDB connection
curl http://localhost:50011/_api/version

# Try manual load
./scripts/load-prod-data.sh
```

### Deployment Fails

```bash
# Check container logs
docker compose --env-file config/.env.production \
  -f deploy/docker-compose.yaml \
  -f deploy/docker-compose.prod.yml logs

# Check container status
docker ps -a

# Rollback: use previous version
./scripts/deploy-tested-images.sh --version v<previous-version>
```

### Migration Fails

```bash
# Check migration logs
cargo run --package stg-rd-migrations --release -- \
  --endpoint http://localhost:50011 \
  --database smacktalk \
  --username root \
  --password <password> \
  --migrations-dir ./migrations/files \
  --dry-run  # Test first

# Restore from backup if needed
# (Use arangorestore with backup from /backups/arangodb/)
```

## Benefits of This Workflow

1. **Zero Delta**: Exact images tested = exact images deployed
2. **No Git Pull in Production**: Only tested artifacts deployed
3. **Faster Deployments**: No build time on production server
4. **Easy Rollback**: Tag previous version and redeploy
5. **Audit Trail**: Version tags show what's deployed
6. **Data Safety**: Volumes persist, data never lost
7. **Migration Safety**: Test migrations before production

## Next Steps

1. **Set up Docker Registry** (optional but recommended)
   - Use Docker Hub, AWS ECR, or private registry
   - Push tested images instead of file transfer

2. **Automate Backups**
   - Schedule regular database backups
   - Keep last N backups for rollback

3. **CI/CD Integration** (future)
   - Automate build on commit
   - Run tests automatically
   - Push to registry on success

4. **Monitoring**
   - Set up health check monitoring
   - Alert on deployment failures
   - Track deployment versions

## Scripts Reference

| Script | Purpose | Usage |
|--------|---------|-------|
| `build-prod-images.sh` | Build production images locally | `./scripts/build-prod-images.sh` |
| `test-prod-containers.sh` | Test production containers locally | `./scripts/test-prod-containers.sh [--run-tests]` |
| `export-tested-images.sh` | Export tested images for deployment | `./scripts/export-tested-images.sh` |
| `backup-prod-db.sh` | Backup production database | `./scripts/backup-prod-db.sh [--local]` |
| `deploy-tested-images.sh` | Deploy tested images to production | `./scripts/deploy-tested-images.sh --version TAG` |
| `load-prod-data.sh` | Load production data snapshot locally | `./scripts/load-prod-data.sh` |

## Questions?

- See individual script files for detailed options
- Check `migrations/README.md` for migration details
- Review `deploy/PRODUCTION_DEPLOYMENT.md` for production setup

