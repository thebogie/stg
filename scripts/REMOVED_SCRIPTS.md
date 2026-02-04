# Removed Scripts - Cleanup Documentation

This document lists scripts that were removed during simplification and what replaced them.

## Removed Scripts (Replaced by New Workflow)

### CI/CD Workflow Scripts (Replaced)
- `run-tests-setup-prod.sh` → **Replaced by `build-test-push.sh`**
  - Old: Built, tested, and pushed in one script but was complex
  - New: Cleaner separation, same functionality

- `deploy-tested-images.sh` → **Replaced by `deploy-production.sh`**
  - Old: Could load from files or Docker Hub, complex logic
  - New: Simple Docker Hub pull and deploy

- `test-and-push-prod.sh` → **Replaced by `build-test-push.sh`**
  - Old: Tested and pushed, but didn't build
  - New: Complete workflow in one script

- `workflow-test-then-deploy.sh` → **Replaced by `build-test-push.sh`**
  - Old: Orchestrated multiple scripts
  - New: Single script does everything

- `pull-and-deploy-prod.sh` → **Replaced by `deploy-production.sh`**
  - Old: Pulled and deployed
  - New: Same functionality, cleaner

- `export-tested-images.sh` → **No longer needed**
  - Old: Exported images to tar.gz files for transfer
  - New: Using Docker Hub registry (no file transfer needed)

- `deploy-prod.sh` → **Replaced by `deploy-production.sh`**
- `deploy-prod-version.sh` → **Replaced by `deploy-production.sh`**
- `deploy-with-migrations.sh` → **Functionality merged into `deploy-production.sh`**

### Test Scripts (Replaced)
- `run-all-tests.sh` → **Replaced by `build-test-push.sh`**
  - Old: Ran all tests but didn't build/push
  - New: Build, test, and push in one workflow

- `run_tests.sh` → **Replaced by `test-dev.sh`**
  - Old: Simple test runner
  - New: Better organized, same functionality

- `test-prod-containers.sh` → **Replaced by `build-test-push.sh`**
  - Old: Tested production containers
  - New: Testing is part of build-test-push workflow

## Current Workflow

### Dev Side
```bash
./scripts/build-test-push.sh
```
- Builds production images
- Runs all tests (unit, integration, e2e)
- Pushes to Docker Hub

### Production Side
```bash
./scripts/deploy-production.sh --version v<commit>-<timestamp>
```
- Pulls from Docker Hub
- Deploys containers
- Runs migrations (optional)

## Kept Scripts (Still Useful)

### Core Scripts
- `build-test-push.sh` - Main dev workflow
- `deploy-production.sh` - Main prod workflow
- `build-prod-images.sh` - Builds production images
- `build-info.sh` - Generates build metadata

### Utility Scripts
- `backup-prod-db.sh` - Database backups
- `load-prod-data.sh` - Load production data for testing
- `test-dev.sh` - Quick unit tests during development
- `test-integration.sh` - Integration tests with testcontainers
- `generate-test-summary.sh` - Test reporting
- `prod-compose.sh` - Docker compose wrapper

### Specialized Scripts
- `test-migrations-workflow.sh` - Test migrations
- `test-migrations-on-existing-data.sh` - Test migrations on existing data
- `create-test-snapshot.sh` - Create test data snapshots
- `restore-test-snapshot.sh` - Restore test data snapshots
- `check-*.sh` - Various check scripts
- `load-*.sh` - Various load scripts
- `start-*.sh`, `stop-*.sh` - Service management scripts
