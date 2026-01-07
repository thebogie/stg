# Test and Deploy Workflow - Complete Guide

**Goal**: Run all tests → Build production images → Test production containers → Deploy to production

## 🚀 Complete Workflow

### Step 1: Run ALL Tests

First, ensure all tests pass before building production images:

```bash
# Run all test suites (backend unit + integration + E2E)
just test-all
```

**What this runs:**
- ✅ Backend unit tests (`just test-backend`) - ~30 seconds
- ✅ Integration tests (`just test-integration`) - ~2-5 minutes  
- ✅ Frontend E2E tests (`just test-frontend-e2e`) - ~2-5 minutes

**Total time**: ~5-10 minutes

**If tests fail**: Fix issues, commit, and re-run tests before proceeding.

---

### Step 2: Build Production Images

Build the exact Docker images that will run in production:

```bash
./scripts/build-prod-images.sh
```

**What this does:**
- Builds frontend and backend Docker images in release mode
- Tags images with version (git commit + timestamp)
- Creates `_build/.build-version` file with version info

**Output:**
- Images: `stg_rd-frontend:latest` and `stg_rd-backend:latest`
- Version file: `_build/.build-version` (contains version tag like `va8b5487-20260106-073414`)

**Time**: ~5-10 minutes

---

### Step 3: Set Up Production Data for Testing (Optional but Recommended)

Before testing production containers, you may want to load production data for realistic testing:

**Option A: Export from Production Server** (If you have access)

```bash
# On production server, export database snapshot
# (This creates a backup file you can transfer to your dev machine)
./scripts/backup-prod-db.sh

# Transfer to your dev machine
scp user@production-server:/path/to/_build/backups/*.tar.gz ./_build/backups/
```

**Option B: Use Existing Backup** (If you already have one)

```bash
# Place your backup file in one of these locations:
# - _build/backups/smacktalk.zip
# - _build/backups/*.tar.gz
# - _build/dumps/*.json or *.json.gz
```

**Option C: Skip Production Data** (Faster, but less realistic)

```bash
# You can test without production data - just use --skip-data-load
./scripts/test-prod-containers.sh --use-test-db --skip-data-load
```

**Note**: The `load-prod-data.sh` script automatically looks for data in:
- `_build/backups/` directory (ArangoDB dumps)
- `_build/dumps/` directory (JSON exports)
- Environment variable `TEST_DATA_DUMP_PATH`

---

### Step 4: Test Production Containers Locally

Test the production images before deploying:

```bash
# Test with separate test database + production data (recommended)
./scripts/test-prod-containers.sh --use-test-db
```

**What this does:**
- Starts production containers locally using the images you just built
- Uses separate test database (`smacktalk_test`) to avoid contaminating data
- **Automatically loads production data** if `load-prod-data.sh` script exists and finds data
- Waits for services to be healthy
- Shows service URLs for manual testing

**What this does:**
- Starts production containers locally using the images you just built
- Uses separate test database to avoid contaminating production data
- Waits for services to be healthy
- Shows service URLs for manual testing

**Alternative options:**
```bash
# Test and also run automated tests against containers
./scripts/test-prod-containers.sh --use-test-db --run-tests

# Test with backup/restore (for migration testing)
./scripts/test-prod-containers.sh --use-test-db --backup-before --restore-after
```

**Manual testing:**
- Backend: http://localhost:${BACKEND_PORT:-50012}
- Frontend: http://localhost:${FRONTEND_PORT:-50013}

**Verify:**
- ✅ Backend health endpoint works
- ✅ Frontend loads correctly
- ✅ Your new feature works as expected

**When done testing:**
```bash
# Stop containers
docker compose --env-file config/.env.production \
  -f deploy/docker-compose.yaml \
  -f deploy/docker-compose.prod.yml \
  down
```

**Time**: ~5-30 minutes (depending on how thorough your testing is)

---

### Step 5: Deploy to Production

Once you've tested and verified everything works:

#### Option A: Using Docker Hub (Recommended)

**On your development machine:**

```bash
# 1. Login to Docker Hub
docker login

# 2. Get the version tag from the build
VERSION=$(cat _build/.build-version | grep VERSION_TAG | cut -d'"' -f2)
echo "Version: $VERSION"

# 3. Tag both images for Docker Hub
docker tag stg_rd-frontend:latest your-username/stg_rd:frontend-$VERSION
docker tag stg_rd-backend:latest your-username/stg_rd:backend-$VERSION

# 4. Push to Docker Hub
docker push your-username/stg_rd:frontend-$VERSION
docker push your-username/stg_rd:backend-$VERSION
```

**On your production server:**

```bash
# 1. Navigate to project directory
cd /path/to/stg_rd

# 2. Login to Docker Hub
docker login

# 3. Pull the tested images
VERSION="va8b5487-20260106-073414"  # Use the version from above
docker pull your-username/stg_rd:frontend-$VERSION
docker pull your-username/stg_rd:backend-$VERSION

# 4. Tag as latest for docker-compose
docker tag your-username/stg_rd:frontend-$VERSION stg_rd-frontend:latest
docker tag your-username/stg_rd:backend-$VERSION stg_rd-backend:latest

# 5. Deploy (this updates running containers)
docker compose --env-file config/.env.production \
  -f deploy/docker-compose.yaml \
  -f deploy/docker-compose.prod.yml \
  up -d

# 6. Verify deployment
curl http://localhost:${BACKEND_PORT:-50012}/health
```

#### Option B: Using Image Export (Alternative)

**On your development machine:**

```bash
# Export tested images
./scripts/export-tested-images.sh

# Transfer to production server
scp _build/artifacts/*.tar.gz* user@production-server:/tmp/
```

**On your production server:**

```bash
# Load images
cd /path/to/stg_rd
VERSION="va8b5487-20260106-073414"  # From export script output
docker load < /tmp/stg_rd-frontend-$VERSION.tar.gz
docker load < /tmp/stg_rd-backend-$VERSION.tar.gz

# Deploy
docker compose --env-file config/.env.production \
  -f deploy/docker-compose.yaml \
  -f deploy/docker-compose.prod.yml \
  up -d
```

---

## 📋 Quick Reference Commands

### All-in-One Workflow

```bash
# 1. Run all tests
just test-all

# 2. Build production images
./scripts/build-prod-images.sh

# 3. (Optional) Load production data for testing
#    Place backup in _build/backups/ or use existing backup

# 4. Test production containers (automatically loads data if available)
./scripts/test-prod-containers.sh --use-test-db

# 5. Deploy (see Step 5 above)
```

### Or Use the Master Script

```bash
# Automated workflow (build → test → export)
./scripts/workflow-test-then-deploy.sh

# Skip tests (not recommended)
./scripts/workflow-test-then-deploy.sh --skip-tests
```

---

## ✅ Checklist Before Deploying

- [ ] All tests pass (`just test-all`)
- [ ] Production images built successfully
- [ ] Production data loaded for testing (optional but recommended)
- [ ] Production containers tested locally
- [ ] New feature works in test environment with production data
- [ ] Backend health check passes
- [ ] Frontend loads correctly
- [ ] Database migrations tested (if applicable)
- [ ] Production backup created (if needed)

---

## 🚨 Important Notes

1. **Always test production containers locally first** - Don't skip Step 3!
2. **Use `--use-test-db` flag** - This prevents contaminating production data during testing
3. **Check version tags** - Make sure you deploy the version you tested
4. **Backup production database** - Before major deployments:
   ```bash
   ./scripts/backup-prod-db.sh
   ```
5. **Test migrations separately** - If you have migrations:
   ```bash
   ./scripts/test-migrations-workflow.sh
   ```

---

## 🆘 Troubleshooting

### Tests Fail
- Fix issues before proceeding
- Re-run specific test suites:
  ```bash
  just test-backend      # Backend only
  just test-integration  # Integration only
  just test-frontend-e2e # E2E only
  ```

### Production Containers Won't Start
- Check `config/.env.production` is configured correctly
- Verify Docker is running
- Check ports aren't already in use

### Deployment Fails
- Verify images exist on Docker Hub (or in exported files)
- Check version tag matches
- Ensure production server has Docker and docker-compose installed

---

## 📚 More Information

- **Daily Workflow**: See `docs/DAILY_WORKFLOW.md` for detailed step-by-step
- **Migration Testing**: See `docs/MIGRATION_TESTING_WORKFLOW.md` if you have migrations
- **Production Deployment**: See `DEPLOY_TO_PRODUCTION.md` for detailed deployment guide

