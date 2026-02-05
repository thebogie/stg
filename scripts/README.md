# Scripts Directory

This directory contains scripts for building, testing, and deploying the application.

## 🚀 Main Workflow Scripts

### Dev Side (Development Machine)

**`build-test-push.sh`** - Complete CI/CD workflow
- Builds production Docker images (with fresh WASM files)
- Starts production containers
- Runs ALL tests (unit, integration, e2e)
- Pushes tested images to Docker Hub

```bash
./scripts/build-test-push.sh
```

**`build-push-only.sh`** - Build and push without tests
- Builds production Docker images (with fresh WASM files)
- Pushes images to Docker Hub
- **Skips all tests** (use when tests already passed or for quick pushes)

```bash
./scripts/build-push-only.sh
```

### Production Side (Production Server)

**`deploy-production.sh`** - Deploy tested containers
- Pulls tested images from Docker Hub
- Deploys containers
- Runs migrations (optional)
- Verifies deployment

```bash
./scripts/deploy-production.sh --version v<commit>-<timestamp>
```

## 🔧 Supporting Scripts

### Build Scripts
- `build-prod-images.sh` - Builds production Docker images (used by build-test-push.sh)
- `build-info.sh` - Generates build metadata (git commit, build date)
- `build-prod.sh` - Local production build (non-Docker, for development)

### Test Scripts
- `test-dev.sh` - Quick unit tests during development
- `test-integration.sh` - Integration tests with testcontainers
- `test-integration-3tier.sh` - Integration tests with 3-tier retry strategy
- `generate-test-summary.sh` - Generate comprehensive test reports

### Utility Scripts
- `backup-prod-db.sh` - Backup production database
- `load-prod-data.sh` - Load production data for testing
- `prod-compose.sh` - Docker compose wrapper for production
- `check-*.sh` - Various check/verification scripts
- `load-*.sh` - Various data loading scripts
- `start-*.sh`, `stop-*.sh` - Service management scripts

### Specialized Scripts
- `test-migrations-workflow.sh` - Test migrations from scratch
- `test-migrations-on-existing-data.sh` - Test migrations on existing data
- `create-test-snapshot.sh` - Create test data snapshots
- `restore-test-snapshot.sh` - Restore test data snapshots

## 📝 Removed Scripts

See `REMOVED_SCRIPTS.md` for a list of scripts that were removed during simplification and what replaced them.

## 🎯 Typical Workflow

### Development
```bash
# Quick unit tests while coding
./scripts/test-dev.sh

# Integration tests
./scripts/test-integration.sh

# Full production build, test, and push
./scripts/build-test-push.sh
```

### Production Deployment
```bash
# On production server
./scripts/deploy-production.sh --version v<commit>-<timestamp>
```
