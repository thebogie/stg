# Migration Guide: Reorganized Project Structure

This guide helps you migrate from the old project structure to the new organized structure.

## What Changed

### 1. Environment Files

**Before:**
- Environment files scattered (root, `../stg_dev/`, `_prod/`)
- Hard to find and manage

**After:**
- All environment files in `config/` directory
- Templates (`env.*.template`) are committed
- Actual files (`.env.*`) are gitignored

**Migration Steps:**
```bash
# 1. Create new environment files from templates
./config/setup-env.sh development
./config/setup-env.sh production

# 2. Copy your existing values to the new files
# Compare your old .env file with config/.env.development
# Update config/.env.development with your actual values

# 3. Remove old environment files (after verifying new ones work)
# rm .env.development  # or wherever your old file was
```

### 2. Docker Compose Files

**Before:**
- `docker-compose.yaml` at root
- `docker-compose.dev.yml` at root
- `_prod/docker-compose.override.yaml` (empty)
- Various scripts with hardcoded paths

**After:**
- All compose files in `deploy/` directory
- Unified `deploy.sh` script
- Clear separation: base, dev, prod

**Migration Steps:**
```bash
# Old way:
docker compose -f docker-compose.yaml -f docker-compose.dev.yml up -d

# New way:
./deploy/deploy.sh --env development --build

# Or for production:
./deploy/deploy.sh --env production --build
```

### 3. Deployment Scripts

**Before:**
- `docker-up.sh` at root
- Old `_prod/stg.sh` (now removed, use `deploy/deploy.sh` instead)
- `scripts/deploy-prod.sh`
- `scripts/build-prod.sh`

**After:**
- Single `deploy/deploy.sh` script
- Works for both dev and prod
- No hardcoded paths

**Migration Steps:**

1. **Update your development workflow:**
   ```bash
   # Old:
   ./docker-up.sh
   
   # New:
   ./deploy/deploy.sh --env development
   ```

2. **Update your production deployment:**
   ```bash
   # Old (if using _prod/stg.sh):
   # cd /home/thebogie/stg/stg_prod
   # ./stg.sh
   
   # New (unified script):
   cd /path/to/stg_rd
   ./deploy/deploy.sh --env production --build
   ```

3. **Update CI/CD scripts:**
   - Replace `docker-compose` commands with `./deploy/deploy.sh`
   - Update environment file paths to `config/.env.production`

### 4. VSCode Tasks

**Before:**
- Tasks referenced `docker-compose.yaml` at root

**After:**
- Tasks use `deploy/deploy.sh` script
- Automatically updated

**No action needed** - tasks are already updated!

## Step-by-Step Migration

### Step 1: Backup Current Setup

```bash
# Backup your current environment files
cp .env.development .env.development.backup 2>/dev/null || true
cp ../stg_dev/.env.development .env.development.backup 2>/dev/null || true
cp _prod/.env.production .env.production.backup 2>/dev/null || true
```

### Step 2: Create New Environment Files

```bash
# Create development environment
./config/setup-env.sh development

# Create production environment (if needed)
./config/setup-env.sh production
```

### Step 3: Migrate Your Values

1. Open your old `.env.development` (wherever it was)
2. Open `config/.env.development`
3. Copy values from old to new, paying attention to:
   - Passwords (ARANGO_ROOT_PASSWORD, ARANGO_PASSWORD)
   - API keys (GOOGLE_LOCATION_API_KEY)
   - Port configurations
   - Volume paths

### Step 4: Test Development Setup

```bash
# Stop old containers (if running)
docker compose down 2>/dev/null || true

# Start with new structure
./deploy/deploy.sh --env development --build

# Verify services are running
./deploy/deploy.sh --env development --status

# Check logs
./deploy/deploy.sh --env development --logs
```

### Step 5: Update Production (When Ready)

```bash
# On production server:
cd /path/to/stg_rd

# Create production environment
./config/setup-env.sh production

# Edit config/.env.production with production values
# - Update VOLUME_PATH to absolute path
# - Update all passwords
# - Update ports if needed
# - Update API keys

# Deploy
./deploy/deploy.sh --env production --build
```

### Step 6: Clean Up Old Files (After Verification)

Once everything works, you can remove old files:

```bash
# Remove old docker-compose files (already moved)
# Remove old scripts (optional, keep as backup initially)
mv docker-up.sh docker-up.sh.old
mv _prod/stg.sh _prod/stg.sh.old

# Remove old environment files (after verifying new ones work)
# Be careful - make sure you've copied all values!
```

## Common Issues

### Issue: "Environment file not found"

**Solution:**
```bash
# Make sure you've created the .env file from template
./config/setup-env.sh development

# Verify it exists
ls -la config/.env.development
```

### Issue: "docker-compose.yaml not found"

**Solution:**
- The compose files are now in `deploy/` directory
- Use `./deploy/deploy.sh` instead of direct docker-compose commands

### Issue: "Build context errors"

**Solution:**
- The docker-compose files now use `context: ..` to build from project root
- This is already configured correctly

### Issue: "Volume paths not working"

**Solution:**
- Check `VOLUME_PATH` in your `.env` file
- For development: `./docker-data` (relative)
- For production: `/var/lib/stg_rd` (absolute)

## Verification Checklist

- [ ] Created `config/.env.development` from template
- [ ] Copied all values from old environment file
- [ ] Tested development deployment: `./deploy/deploy.sh --env development --build`
- [ ] Services start correctly
- [ ] Can access frontend and backend
- [ ] Database connections work
- [ ] Created `config/.env.production` (if deploying to prod)
- [ ] Updated production deployment process
- [ ] Updated CI/CD scripts (if applicable)
- [ ] VSCode tasks work correctly
- [ ] Removed or backed up old files

## Getting Help

If you encounter issues:

1. Check `PROJECT_STRUCTURE.md` (in this directory) for structure overview
2. Check `DEVELOPMENT_SETUP.md` (in this directory) for development setup
3. Review the deploy script: `./deploy/deploy.sh --help`
4. Check environment setup: `./config/setup-env.sh --help`

## Rollback

If you need to rollback:

```bash
# Restore old docker-compose files (if you backed them up)
# Restore old environment files from backup
# The new deploy/deploy.sh script works for both dev and prod
```

However, the new structure is designed to be backward compatible - you can keep both old and new files during transition.

