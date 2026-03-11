# Production Directory Structure

This document describes what files and directories should be present on your production server.

## Required Files and Directories

### ✅ **Required for Deployment**

```
production-server:/path/to/stg/
├── Cargo.toml                    # Required: Workspace root identifier
├── Cargo.lock                   # Required: Dependency lock file (for migrations)
│
├── config/                      # Required: Configuration files
│   ├── .env.production          # Required: Production environment (gitignored, create manually)
│   ├── env.production.template  # Optional: Template (for reference)
│   └── setup-env.sh             # Optional: Setup script
│
├── deploy/                      # Required: Docker Compose configuration
│   ├── docker-compose.yaml      # Required: Base configuration
│   ├── docker-compose.prod.yml  # Required: Production overrides
│   ├── docker-compose.stg_prod.yml  # Required: Production network config
│   └── deploy.sh                # Optional: Alternative deployment script
│
├── migrations/                  # Required: Database migrations
│   └── files/                   # Required: Migration files (if using migrations)
│
├── scripts/                     # Required: Deployment scripts
│   ├── deploy-tested-images.sh  # Required: Main deployment script
│   ├── backup-prod-db.sh        # Optional: Database backup script
│   └── build-info.sh           # Optional: Version info script
│
└── shared/                      # Required: Shared types (for migrations)
    └── (Rust package structure)
```

### ⚠️ **Conditionally Required**

- **`migrations/` package**: Only needed if you run migrations from the host
- **`Cargo.lock`**: Only needed if running migrations (cargo needs it)
- **`shared/` package**: Only needed for migrations that use shared types

### ❌ **NOT Required (Code is in Docker Images)**

Since you're deploying pre-built Docker images, you **do NOT need**:

```
❌ backend/          # Source code is in Docker image
❌ frontend/         # Source code is in Docker image  
❌ target/           # Build artifacts are in Docker images
❌ node_modules/     # Dependencies are in Docker images
❌ _build/           # Build artifacts
❌ test-results/     # Test results
❌ docs/             # Documentation (optional, nice to have)
❌ testing/          # Test code
❌ dataload/         # Data loading utilities (unless you use them)
```

## Minimal Production Setup

### Option 1: Full Git Clone (Recommended)

Clone the entire repository - this is the simplest approach:

```bash
# On production server
cd /path/to/
git clone <your-repo-url> stg
cd stg

# Create production environment
./config/setup-env.sh production
nano config/.env.production  # Edit with production values

# Deploy (using pre-built images)
./scripts/deploy-tested-images.sh --image-dir /tmp
```

**Pros:**
- ✅ Simple - just clone and deploy
- ✅ All scripts and configs available
- ✅ Easy to update (git pull)
- ✅ Can run migrations easily

**Cons:**
- ⚠️ Includes source code (not needed, but harmless)
- ⚠️ Larger directory size

### Option 2: Minimal Directory (Advanced)

Create only the required files:

```bash
# On production server
mkdir -p /path/to/stg/{config,deploy,migrations/files,scripts,shared/src}
cd /path/to/stg

# Copy required files
# (You'd need to manually copy or use rsync with exclusions)
```

**Pros:**
- ✅ Minimal footprint
- ✅ No unnecessary files

**Cons:**
- ❌ Complex to set up
- ❌ Hard to maintain
- ❌ Can't easily run migrations
- ❌ Difficult to update

## Recommended Approach: Git Clone with .dockerignore

The best approach is to **clone the full repository** but understand that:

1. **Source code is harmless** - it's not used (Docker images contain everything)
2. **Git clone is standard** - easy to update with `git pull`
3. **All scripts available** - migrations, backups, etc. work out of the box
4. **Docker ignores source** - `.dockerignore` prevents copying source into images

## What Gets Used vs Ignored

### During Deployment (deploy-tested-images.sh)

**Uses:**
- ✅ `Cargo.toml` - Verifies project root
- ✅ `config/.env.production` - Environment configuration
- ✅ `deploy/*.yml` - Docker Compose files
- ✅ `migrations/files/` - Migration files (if running migrations)
- ✅ `scripts/deploy-tested-images.sh` - Deployment script
- ✅ `Cargo.lock` + workspace - For running migrations with `cargo run`

**Ignores:**
- ❌ `backend/` - Code is in Docker image
- ❌ `frontend/` - Code is in Docker image
- ❌ `target/` - Build artifacts in Docker image
- ❌ `node_modules/` - Dependencies in Docker image

### During Docker Build (if building on production)

If you build images on production (not recommended), Docker uses `.dockerignore`:
- ✅ Copies: `Cargo.toml`, `Cargo.lock`, source code
- ❌ Ignores: `target/`, `node_modules/`, `.git/`, `docs/`, `_build/`

## Directory Size Comparison

| Approach | Size | Notes |
|----------|------|-------|
| Full Git Clone | ~50-100 MB | Includes all source code |
| Minimal Setup | ~5-10 MB | Only required files |
| Docker Images | ~500 MB - 1 GB | Images stored separately |

**Note:** Docker images are stored in Docker's storage, not in your project directory.

## Production Server Checklist

When setting up production, ensure you have:

- [ ] Git repository cloned (or minimal files copied)
- [ ] `config/.env.production` created and configured
- [ ] Docker and Docker Compose installed
- [ ] Required ports available (50001, 50002, 50003, 63791)
- [ ] Volume directory created (`/var/lib/stg_rd` or as configured)
- [ ] Network created (`docker network create stg_prod`)
- [ ] Docker images loaded (from tar.gz files or registry)

## Updating Production

### Method 1: Git Pull (Recommended)

```bash
cd /path/to/stg
git pull origin main
./scripts/deploy-tested-images.sh --image-dir /tmp
```

### Method 2: Rsync (If not using Git)

```bash
# From development machine
rsync -avz --exclude='target/' --exclude='node_modules/' \
  --exclude='_build/' --exclude='.git/' \
  ./ user@production:/path/to/stg/
```

## Security Considerations

1. **Never commit `.env.production`** - It contains secrets
2. **Restrict file permissions** - `chmod 600 config/.env.production`
3. **Use Docker secrets** - For highly sensitive deployments
4. **Limit access** - Only deployment user should have access
5. **Source code is safe** - It's not executed, only in Docker images

## Summary

**For production, you need:**
- ✅ Git repository (full clone is fine)
- ✅ `config/.env.production` (with your secrets)
- ✅ `deploy/` directory (Docker Compose files)
- ✅ `scripts/` directory (deployment scripts)
- ✅ `migrations/` directory (if using migrations)
- ✅ `Cargo.toml` and workspace (for migrations)

**You don't need (but harmless if present):**
- ⚠️ Source code (`backend/`, `frontend/`) - in Docker images
- ⚠️ Build artifacts (`target/`, `_build/`) - in Docker images
- ⚠️ Dependencies (`node_modules/`) - in Docker images

**Best Practice:** Clone the full repository. It's simple, standard, and the unused files don't hurt anything.

