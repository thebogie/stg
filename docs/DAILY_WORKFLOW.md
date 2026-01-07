# Daily Development Workflow (80% Case)

This is your typical day-to-day workflow for regular code changes, bug fixes, and feature additions **without migrations**.

## Overview

**Goal**: Make code changes → Test them → Deploy exact tested artifacts to production

**Time**: ~15-30 minutes for a typical deployment

**Frequency**: As often as you want (daily, multiple times per day)

## Complete Workflow

### Step 1: Make Your Code Changes

**What you do:**
- Edit code in your IDE
- Fix bugs, add features, refactor
- Commit changes to git

**Why:**
- This is your normal development work
- No special process needed here

**Example:**
```bash
# Make changes in your editor
vim backend/src/routes.rs  # or use your IDE

# Commit when ready
git add .
git commit -m "Fix user authentication bug"
```

---

### Step 2: Build Production Images

**What you do:**
```bash
./scripts/build-prod-images.sh
```

**What it does:**
- Builds frontend and backend Docker images using production configuration
- Compiles in release mode (optimized, no debug symbols)
- Tags images with version (git commit + timestamp)
- Creates `_build/.build-version` file with version info

**Why:**
- **Production parity**: Builds exactly what will run in production
- **Version tracking**: Tags images so you know what you tested
- **Optimization**: Release builds are faster and smaller
- **Reproducibility**: Same code = same image every time

**Output:**
```
✅ Images built successfully
✅ Images tagged: 
  Frontend: stg_rd-frontend:vabc123-20250120-143022
  Backend: stg_rd-backend:vabc123-20250120-143022
```

**Time**: ~5-10 minutes (depends on code changes)

---

### Step 3: Test Production Containers Locally

**What you do:**
```bash
./scripts/test-prod-containers.sh --use-test-db
```

**What it does:**
- Starts production containers locally
- Uses separate test database (`smacktalk_test`) to avoid contaminating data
- Loads production data snapshot (if available)
- Waits for services to be healthy
- Shows you service URLs

**Why:**
- **Test exact production setup**: Same containers, same config
- **Data isolation**: Uses test database so you don't contaminate production data
- **Catch issues early**: Find problems before deploying to production
- **Confidence**: Know your code works before deploying

**Options:**
- `--use-test-db`: Use separate test database (recommended)
- `--skip-data-load`: Skip loading production data (faster, but less realistic)
- `--run-tests`: Run automated test suite

**What you test:**
- Manual testing in browser (http://localhost:${FRONTEND_PORT})
- API testing (http://localhost:${BACKEND_PORT})
- Integration testing
- Performance testing
- Any specific scenarios related to your changes

**Time**: ~2-5 minutes to start, then however long you need to test

**Example:**
```bash
# Load environment to get correct ports
source scripts/load-env.sh production

# Start containers
./scripts/test-prod-containers.sh --use-test-db

# Test in browser (uses port from .env file)
# Open http://localhost:${FRONTEND_PORT}
# Test your changes

# Test API (uses port from .env file)
curl http://localhost:${BACKEND_PORT}/api/users

# When done testing
docker compose --env-file config/.env.${RUST_ENV} \
  -f deploy/docker-compose.yaml \
  -f deploy/docker-compose.prod.yml \
  down
```

---

### Step 4: Export Tested Images

**What you do:**
```bash
./scripts/export-tested-images.sh
```

**What it does:**
- Exports the tested Docker images to tar.gz files
- Creates checksums for verification
- Saves deployment instructions
- Stores everything in `_build/artifacts/` directory

**Why:**
- **Artifact preservation**: Save the exact images you tested
- **Transfer to production**: Files ready to move to production server
- **Verification**: Checksums ensure images aren't corrupted
- **Audit trail**: Know exactly what was tested and deployed

**Output:**
```
✅ Images exported successfully!
Files in _build/artifacts/:
  frontend-vabc123-20250120-143022.tar.gz (45MB)
  backend-vabc123-20250120-143022.tar.gz (28MB)
  deploy-info-vabc123-20250120-143022.txt
  *.sha256 (checksums)
```

**Time**: ~1-2 minutes

---

### Step 5: Deploy to Production

**See the complete deployment guide**: **[DEPLOY_TO_PRODUCTION.md](../DEPLOY_TO_PRODUCTION.md)**

**Quick summary:**
1. Push images to Docker Hub (single private repository with tags)
2. Pull images on production server
3. Tag images for deployment
4. Run deployment script with `--skip-load` flag

**Full details**: See `DEPLOY_TO_PRODUCTION.md` in the project root for complete step-by-step instructions.
- **Health checks**: Verifies services are running

**What happens to data:**
- **ArangoDB data**: Stays in `/var/lib/stg_rd/arango_data/` volume
- **Redis data**: Stays in `/var/lib/stg_rd/redis_data/` volume
- **No data loss**: Volumes persist across container deployments

**Time**: ~2-3 minutes

**Output:**
```
✅ Images loaded
✅ Services deployed
✅ Backend health check passed
✅ Frontend is accessible
✅ Deployment completed!
```

---

### Step 7: Verify Production Deployment

**What you do:**
```bash
# Check container status
docker compose --env-file config/.env.production \
  -f deploy/docker-compose.yaml \
  -f deploy/docker-compose.prod.yml \
  ps

# Check logs
docker compose --env-file config/.env.production \
  -f deploy/docker-compose.yaml \
  -f deploy/docker-compose.prod.yml \
  logs -f

# Load environment to get correct ports
source scripts/load-env.sh production

# Test endpoints (uses ports from .env file)
curl http://localhost:${BACKEND_PORT}/health
curl http://localhost:${FRONTEND_PORT}
```

**Why:**
- **Confidence**: Verify everything is working
- **Quick feedback**: Catch issues immediately
- **Monitoring**: Watch logs for any errors

**Time**: ~1-2 minutes

---

## Quick Reference: One-Liner Workflow

For experienced users, here's the condensed version:

```bash
# Development machine
./scripts/build-prod-images.sh && \
./scripts/test-prod-containers.sh --use-test-db && \
./scripts/export-tested-images.sh && \
scp _build/artifacts/*.tar.gz* user@production:/tmp/

# Production server (SSH in)
cd /path/to/stg_rd && \
./scripts/deploy-tested-images.sh --version $(cat _build/.build-version | grep VERSION_TAG | cut -d= -f2) --image-dir /tmp
```

---

## Typical Timeline

| Step | Time | What You're Doing |
|------|------|-------------------|
| 1. Code changes | 30min-2hr | Normal development |
| 2. Build images | 5-10min | Automated, grab coffee |
| 3. Test locally | 5-30min | Manual testing, verify changes |
| 4. Export images | 1-2min | Automated |
| 5. Transfer | 2-5min | Automated, grab coffee |
| 6. Deploy | 2-3min | Automated |
| 7. Verify | 1-2min | Quick check |
| **Total** | **~15-50min** | Mostly automated |

---

## Why This Workflow Works

### 1. **Zero Delta Between Test and Production**
- You test production containers
- You deploy the exact same containers
- No surprises

### 2. **No Git Pull in Production**
- Production doesn't need source code
- Only tested artifacts deployed
- Faster, cleaner deployments

### 3. **Data Safety**
- Data persists in Docker volumes
- No data loss during deployments
- Easy rollback if needed

### 4. **Fast Iteration**
- Build once, test, deploy
- No rebuild on production server
- Quick feedback loop

### 5. **Confidence**
- Tested before deployment
- Exact artifacts deployed
- Easy to verify

---

## Common Variations

### Quick Bug Fix (No Data Needed)

```bash
# Skip data loading for faster testing
./scripts/build-prod-images.sh
./scripts/test-prod-containers.sh --skip-data-load
./scripts/export-tested-images.sh
# ... deploy
```

### Feature Testing (Need Production Data)

```bash
# Load production data first
./scripts/load-prod-data.sh  # One-time setup
./scripts/build-prod-images.sh
./scripts/test-prod-containers.sh --use-test-db
./scripts/export-tested-images.sh
# ... deploy
```

### Multiple Deployments Per Day

```bash
# Just repeat the workflow
# Each deployment is independent
# Previous deployments don't affect new ones
```

---

## Troubleshooting

### Build Fails
- Check compilation errors
- Fix code issues
- Rebuild

### Tests Fail Locally
- Fix bugs
- Rebuild and retest
- Don't deploy until tests pass

### Deployment Fails
- Check logs: `docker compose logs`
- Verify images loaded: `docker images`
- Check health: `source scripts/load-env.sh production && curl http://localhost:${BACKEND_PORT}/health`

### Need to Rollback
```bash
# Deploy previous version
./scripts/deploy-tested-images.sh --version v<previous-version> --image-dir /tmp
```

---

## Summary

**Your typical workflow (80% case):**

1. **Code** → Make changes (normal development)
2. **Build** → `./scripts/build-prod-images.sh` (5-10min)
3. **Test** → `./scripts/test-prod-containers.sh --use-test-db` (5-30min)
4. **Export** → `./scripts/export-tested-images.sh` (1-2min)
5. **Transfer** → `scp _build/artifacts/*.tar.gz* production:/tmp/` (2-5min)
6. **Deploy** → `./scripts/deploy-tested-images.sh --version v<version>` (2-3min)
7. **Verify** → Check logs and endpoints (1-2min)

**Total time**: ~15-50 minutes (mostly automated)

**Key benefits:**
- ✅ Test exact production setup
- ✅ Deploy exact tested artifacts
- ✅ No git pull in production
- ✅ Data always safe
- ✅ Fast, reliable deployments

