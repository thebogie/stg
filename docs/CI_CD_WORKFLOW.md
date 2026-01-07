# Complete CI/CD Workflow: Hybrid Development to Production Release

This document describes the complete workflow from hybrid development through to production release, ensuring all generated objects are properly managed and the entire CI/CD pipeline works correctly.

## Overview

The workflow consists of three main phases:
1. **Hybrid Development** - Local development with debugger support
2. **Build & Test** - Production build and local testing
3. **Production Release** - Deploy tested artifacts to production

All generated objects are consolidated in the `_build/` directory, which is excluded from git.

## Phase 1: Hybrid Development

### Setup

**For Development:**
```bash
# 1. Start dependencies (ArangoDB + Redis in Docker)
#    Uses config/.env.development by default
./scripts/setup-hybrid-dev.sh

# 2. Start backend in VSCode
#    Debug panel → "Debug Backend (Hybrid Dev)" → F5
#    (Uses config/.env.development automatically)

# 3. Start frontend
#    ./scripts/start-frontend.sh
#    (Uses config/.env.development automatically)
```

**For Production:**
```bash
# 1. Start dependencies
#    RUST_ENV=production ./scripts/setup-hybrid-dev.sh
#    Or: source scripts/load-env.sh production && ./scripts/setup-hybrid-dev.sh

# 2. Start backend
#    source scripts/load-env.sh production
#    cargo run --package backend --bin backend

# 3. Start frontend
#    source scripts/load-env.sh production
#    ./scripts/start-frontend.sh
```

**Note**: All scripts use `scripts/load-env.sh` which automatically loads the correct environment file (`config/.env.development` or `config/.env.production`) based on the `RUST_ENV` variable or explicit argument.

### Development Workflow

1. **Make code changes** in your IDE
2. **Backend**: Auto-reloads when you restart debugger
3. **Frontend**: Hot reloads automatically via Trunk
4. **Test locally** with production-like data
5. **Debug with breakpoints** in both frontend and backend

### Generated Objects During Development

- **Rust build artifacts**: `_build/target/` (via `.cargo/config.toml`)
- **Frontend dist**: `_build/frontend-dist/` (via `frontend/Trunk.toml`)
- **Test results**: `_build/test-results/`, `_build/coverage/`
- **Data dumps**: `_build/dumps/` (if exporting data)

**Note**: All generated objects are in `_build/` and excluded from git.

## Phase 2: Build & Test

### Step 1: Build Production Images

```bash
./scripts/build-prod-images.sh
```

**What it does:**
- Builds frontend and backend Docker images in release mode
- Tags images with version (git commit + timestamp)
- Creates `_build/.build-version` with version info

**Output:**
- Images: `stg_rd-frontend:v<commit>-<timestamp>`
- Version file: `_build/.build-version`
- All build artifacts in `_build/target/`

### Step 2: Test Production Containers

```bash
# Option A: Use test database (recommended for regular testing)
./scripts/test-prod-containers.sh --use-test-db

# Option B: Test with production database (for final validation)
./scripts/test-prod-containers.sh --backup-before --restore-after
```

**What it does:**
- Starts production containers locally
- Loads production data (from `_build/backups/` or `_build/dumps/`)
- Waits for services to be healthy
- Provides URLs for manual testing

**Test endpoints** (ports from your `config/.env.production`):
- Frontend: http://localhost:${FRONTEND_PORT}
- Backend: http://localhost:${BACKEND_PORT}
- ArangoDB: http://localhost:${ARANGODB_PORT}

**To check your ports:**
```bash
source scripts/load-env.sh production
echo "Frontend: http://localhost:${FRONTEND_PORT}"
echo "Backend: http://localhost:${BACKEND_PORT}"
echo "ArangoDB: http://localhost:${ARANGODB_PORT}"
```

### Step 3: Run Automated Tests

```bash
# Run all tests
just test-all

# Or specific test suites
just test-backend
just test-frontend-e2e
just test-backend-coverage
```

**Test artifacts:**
- Test results: `_build/test-results.xml`
- Coverage reports: `_build/coverage/html/`
- LCOV file: `_build/lcov.info`
- Playwright reports: `_build/playwright-report/`

### Step 4: Export Tested Images

```bash
./scripts/export-tested-images.sh
```

**What it does:**
- Exports tested Docker images to tar.gz files
- Creates checksums for verification
- Saves deployment instructions

**Output in `_build/artifacts/`:**
- `frontend-v<version>.tar.gz`
- `backend-v<version>.tar.gz`
- `deploy-info-<version>.txt`
- `*.sha256` (checksums)

## Phase 3: Production Release

**See the complete deployment guide**: **[DEPLOY_TO_PRODUCTION.md](../DEPLOY_TO_PRODUCTION.md)**

This is the authoritative guide for deploying to production using Docker Hub with a single private repository.

**Quick summary:**
1. Push tested images to Docker Hub (single repository with tags)
2. Pull images on production server
3. Tag images for deployment
4. Deploy using `deploy-tested-images.sh` with `--skip-load` flag

**Full details**: See `DEPLOY_TO_PRODUCTION.md` in the project root for complete step-by-step instructions, troubleshooting, and rollback procedures.

## Complete Workflow Example

### Scenario: Regular Feature Development

```bash
# === PHASE 1: Hybrid Development ===

# 1. Setup hybrid dev environment (one command)
./scripts/setup-hybrid-dev.sh
# This starts containers AND loads data (if available)

# 2. Develop in VSCode with debugger
#    - Backend: Debug panel → F5
#    - Frontend: Tasks → "frontend: trunk serve"
#    - Make code changes, test locally

# === PHASE 2: Build & Test ===

# 3. Build production images
./scripts/build-prod-images.sh
# Creates: _build/.build-version, images tagged with version

# 4. Test production containers
./scripts/test-prod-containers.sh --use-test-db
# Test in browser: http://localhost:50013
# Test API: http://localhost:50012

# 5. Run automated tests
just test-all
# Creates: _build/test-results.xml, _build/coverage/

# 6. Export tested images
./scripts/export-tested-images.sh
# Creates: _build/artifacts/*.tar.gz

# === PHASE 3: Production Release ===

# 7. Transfer to production
scp _build/artifacts/*.tar.gz* user@production:/tmp/

# 8. Deploy (on production server)
ssh production-server
cd /path/to/stg_rd
./scripts/backup-prod-db.sh
./scripts/deploy-tested-images.sh --version v<version> --image-dir /tmp

# 9. Verify
curl http://production-server:50012/health
```

### Scenario: Deployment with Migrations

```bash
# === PHASE 1: Development ===
# (Same as above)

# === PHASE 2: Build & Test ===

# 1. Build production images
./scripts/build-prod-images.sh

# 2. Test migrations from scratch
./scripts/test-migrations-workflow.sh
# This wipes database and runs migrations from scratch

# 3. Test migrations on existing data
./scripts/test-migrations-on-existing-data.sh ./_build/backups/latest-backup.tar.gz
# This restores backup and runs migrations on existing data

# 4. Test production containers
./scripts/test-prod-containers.sh --use-test-db

# 5. Export tested images
./scripts/export-tested-images.sh

# === PHASE 3: Production Release ===

# 6. Transfer to production
scp _build/artifacts/*.tar.gz* user@production:/tmp/

# 7. Deploy with migrations (on production server)
ssh production-server
cd /path/to/stg_rd
./scripts/backup-prod-db.sh  # Backup before migrations
./scripts/deploy-with-migrations.sh --version v<version> --image-dir /tmp
```

## Directory Structure

All generated objects are in `_build/`:

```
_build/
├── target/              # Rust build artifacts (via .cargo/config.toml)
├── frontend-dist/        # Frontend build output (via Trunk.toml)
├── test-results/        # Test results
│   ├── test-results.xml # JUnit XML
│   └── e2e-results.xml  # Playwright results
├── coverage/            # Coverage reports
│   └── html/            # HTML coverage reports
├── playwright-report/   # Playwright HTML reports
├── lcov.info           # LCOV coverage file
├── artifacts/          # Docker image exports
│   ├── frontend-v*.tar.gz
│   ├── backend-v*.tar.gz
│   └── deploy-info-*.txt
├── backups/            # Database backups
│   ├── migration-test/
│   ├── test-backups/
│   └── test-snapshots/
├── dumps/              # Data dumps
│   ├── dump.json
│   └── dump.sanitized.json.gz
└── .build-version      # Build metadata
```

## Key Benefits

1. **All generated objects in one place**: Everything in `_build/`, easy to exclude from git
2. **Zero delta between test and production**: Exact same images tested and deployed
3. **No git pull in production**: Only tested artifacts deployed
4. **Fast iteration**: Hybrid dev for fast development, production builds for testing
5. **Data safety**: Backups before deployments, volumes persist data
6. **Migration safety**: Test migrations in both scenarios before production

## Automation Opportunities

### CI/CD Pipeline (Future)

```yaml
# Example GitHub Actions workflow
name: Build and Test

on:
  push:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      
      # Build production images
      - run: ./scripts/build-prod-images.sh
      
      # Run tests
      - run: just test-all
      
      # Export images
      - run: ./scripts/export-tested-images.sh
      
      # Upload artifacts
      - uses: actions/upload-artifact@v3
        with:
          name: docker-images
          path: _build/artifacts/
      
      # Push to registry (if configured)
      - run: |
          docker tag stg_rd-frontend:v${{ github.sha }} registry/stg_rd-frontend:v${{ github.sha }}
          docker push registry/stg_rd-frontend:v${{ github.sha }}
```

### Automated Deployment

```bash
# Master workflow script
./scripts/workflow-test-then-deploy.sh

# This orchestrates:
# 1. Build production images
# 2. Test containers
# 3. Export images
# 4. (Optionally) Transfer and deploy
```

## Troubleshooting

### Build Fails

```bash
# Check for compilation errors
cargo build --release

# Check Docker build logs
docker compose build --progress=plain
```

### Tests Fail

```bash
# Run tests with more output
RUST_LOG=debug just test-all

# Check test results
cat _build/test-results.xml
```

### Deployment Fails

```bash
# Check container logs
docker compose logs

# Verify images loaded
docker images | grep stg_rd

# Rollback to previous version
./scripts/deploy-tested-images.sh --version v<previous-version>
```

## Summary

**Complete workflow:**
1. **Develop** → Hybrid dev with debugger (fast iteration)
2. **Build** → Production images (`_build/`)
3. **Test** → Local testing with production containers
4. **Export** → Tested images to `_build/artifacts/`
5. **Transfer** → Images to production server
6. **Deploy** → Tested images to production
7. **Verify** → Check deployment health

**All generated objects in `_build/`** - excluded from git, easy to clean up, organized and consistent.

