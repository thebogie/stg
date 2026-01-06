# Deploy Tested Docker Containers to Production

This guide walks you through deploying your tested Docker containers from your development machine to your production server using Docker Hub with a single private repository.

## Prerequisites

- ✅ All tests have passed on your development machine
- ✅ Docker images have been built and tested
- ✅ Production server has Docker and Docker Compose installed
- ✅ Docker Hub account (free tier allows 1 private repository)
- ✅ Production server has the project repository cloned

## Step-by-Step Deployment Process

### Step 1: Build and Tag Images (On Development Machine)

First, ensure your production images are built with proper version tags:

```bash
# From your project root directory
cd /home/thebogie/work/stg

# Build production images (creates version tags)
./scripts/build-prod-images.sh
```

**What this does:**
- Builds frontend and backend Docker images in release mode
- Tags images with version (git commit + timestamp)
- Creates `_build/.build-version` file with version info

**Output:**
- Images: `stg_rd-frontend:latest` and `stg_rd-backend:latest`
- Version file: `_build/.build-version` (contains version tag like `va8b5487-20260106-073414`)

**Note:** If you already ran E2E tests, the images may already exist. The build script will reuse them quickly due to Docker cache.

### Step 2: Push Images to Docker Hub (On Development Machine)

Push your tested images to Docker Hub using a single private repository with descriptive tags:

```bash
# Login to Docker Hub
docker login

# Get the version tag from the build
VERSION=$(cat _build/.build-version | grep VERSION_TAG | cut -d'"' -f2)
echo "Version: $VERSION"

# Tag both images to the same repository with descriptive tags
docker tag stg_rd-frontend:latest your-username/stg_rd:frontend-$VERSION
docker tag stg_rd-backend:latest your-username/stg_rd:backend-$VERSION

# Push to Docker Hub
docker push your-username/stg_rd:frontend-$VERSION
docker push your-username/stg_rd:backend-$VERSION
```

**Example:**
```bash
docker tag stg_rd-frontend:latest therealbogie/stg_rd:frontend-va8b5487-20260106-073414
docker tag stg_rd-backend:latest therealbogie/stg_rd:backend-va8b5487-20260106-073414
docker push therealbogie/stg_rd:frontend-va8b5487-20260106-073414
docker push therealbogie/stg_rd:backend-va8b5487-20260106-073414
```

**Important:** 
- Make sure to set the `stg_rd` repository to **Private** on Docker Hub if you want to keep your images private
- The free tier allows 1 private repository, which is why we use a single repository with different tags

### Step 3: Prepare Production Server

**On your production server:**

1. **Navigate to project directory:**
   ```bash
   cd /home/thebogie/stg/repo  # Or wherever your repo is located
   ```

2. **Ensure production environment is configured:**
   ```bash
   # If not already done, create production environment file
   ./config/setup-env.sh production
   
   # Edit with your production values
   nano config/.env.production
   ```

   **Important settings to verify:**
   - `VOLUME_PATH` - Absolute path for persistent data (e.g., `/home/thebogie/stg`)
   - `ARANGO_ROOT_PASSWORD` - Strong password for ArangoDB
   - `ARANGO_PASSWORD` - Strong password for database user
   - `REDIS_PASSWORD` - Redis password (if using)
   - Port configurations (FRONTEND_PORT, BACKEND_PORT, etc.)
   - Database connection URLs
   - API keys (Google, BGG, etc.)

3. **Ensure Docker network exists:**
   ```bash
   # Create the production network if it doesn't exist
   docker network create stg_prod 2>/dev/null || true
   ```

### Step 4: Pull and Deploy Images (On Production Server)

**On your production server:**

1. **Pull images from Docker Hub:**
   ```bash
   # Login to Docker Hub
   docker login

   # Pull the images (replace with your actual version tag)
   VERSION="va8b5487-20260106-073414"  # Get this from your build
   docker pull your-username/stg_rd:frontend-$VERSION
   docker pull your-username/stg_rd:backend-$VERSION
   ```

2. **Tag images for deployment:**
   ```bash
   # Tag them to the expected names for deployment
   docker tag your-username/stg_rd:frontend-$VERSION stg_rd-frontend:latest
   docker tag your-username/stg_rd:backend-$VERSION stg_rd-backend:latest
   ```

3. **Deploy using the deployment script:**
   ```bash
   # Deploy with the version tag (--skip-load because images are already loaded from Docker Hub)
   ./scripts/deploy-tested-images.sh --version $VERSION --skip-load --skip-backup
   ```

   **Note:** 
   - The `--skip-load` flag tells the script that images are already loaded from Docker Hub (no tar.gz files needed)
   - The `--skip-backup` flag is optional - remove it if you want to backup before deployment

   **What this script does:**
   - Verifies images are available
   - Creates database backup (unless `--skip-backup` is used)
   - Stops old containers
   - Starts new containers with tested images
   - Runs migrations (optional, can skip with `--skip-migrations`)
   - Verifies deployment health

4. **Alternative: Manual deployment (if you prefer more control):**
   ```bash
   # Set environment variables
   export ENV_FILE=config/.env.production
   
   # Stop existing containers
   docker compose \
       --env-file config/.env.production \
       -f deploy/docker-compose.yaml \
       -f deploy/docker-compose.prod.yml \
       -f deploy/docker-compose.stg_prod.yml \
       down
   
   # Start new containers (uses stg_rd-frontend:latest and stg_rd-backend:latest)
   docker compose \
       --env-file config/.env.production \
       -f deploy/docker-compose.yaml \
       -f deploy/docker-compose.prod.yml \
       -f deploy/docker-compose.stg_prod.yml \
       up -d
   ```

### Step 5: Verify Deployment

**Check container status:**
```bash
docker compose \
    --env-file config/.env.production \
    -f deploy/docker-compose.yaml \
    -f deploy/docker-compose.prod.yml \
    -f deploy/docker-compose.stg_prod.yml \
    ps
```

**Check logs:**
```bash
# All services
docker compose \
    --env-file config/.env.production \
    -f deploy/docker-compose.yaml \
    -f deploy/docker-compose.prod.yml \
    -f deploy/docker-compose.stg_prod.yml \
    logs -f

# Specific service
docker compose \
    --env-file config/.env.production \
    -f deploy/docker-compose.yaml \
    -f deploy/docker-compose.prod.yml \
    -f deploy/docker-compose.stg_prod.yml \
    logs -f backend
```

**Test health endpoints:**
```bash
# Backend health check (adjust port from your .env.production)
curl http://localhost:50002/health

# Frontend (adjust port from your .env.production)
curl http://localhost:50001/

# Detailed health check
curl http://localhost:50002/health/detailed
```

### Step 6: Post-Deployment Tasks

1. **Run migrations (if needed):**
   ```bash
   # If you skipped migrations during deployment
   cargo run --package stg-rd-migrations --release -- \
       --endpoint "http://localhost:50003" \
       --database "smacktalk" \
       --username "root" \
       --password "<your-password>" \
       --migrations-dir migrations/files
   ```

2. **Clean up old images (optional):**
   ```bash
   # Remove old/unused images to free up space
   docker image prune -a
   ```

## Quick Reference

### Development Machine Commands
```bash
# Build production images
./scripts/build-prod-images.sh

# Get version tag
VERSION=$(cat _build/.build-version | grep VERSION_TAG | cut -d'"' -f2)

# Tag and push to Docker Hub
docker login
docker tag stg_rd-frontend:latest your-username/stg_rd:frontend-$VERSION
docker tag stg_rd-backend:latest your-username/stg_rd:backend-$VERSION
docker push your-username/stg_rd:frontend-$VERSION
docker push your-username/stg_rd:backend-$VERSION
```

### Production Server Commands
```bash
# Pull images
docker login
VERSION="va8b5487-20260106-073414"  # Your version tag
docker pull your-username/stg_rd:frontend-$VERSION
docker pull your-username/stg_rd:backend-$VERSION

# Tag for deployment
docker tag your-username/stg_rd:frontend-$VERSION stg_rd-frontend:latest
docker tag your-username/stg_rd:backend-$VERSION stg_rd-backend:latest

# Deploy
cd /home/thebogie/stg/repo
./scripts/deploy-tested-images.sh --version $VERSION --skip-load --skip-backup

# Check status
docker compose --env-file config/.env.production \
    -f deploy/docker-compose.yaml \
    -f deploy/docker-compose.prod.yml \
    -f deploy/docker-compose.stg_prod.yml \
    ps

# View logs
docker compose --env-file config/.env.production \
    -f deploy/docker-compose.yaml \
    -f deploy/docker-compose.prod.yml \
    -f deploy/docker-compose.stg_prod.yml \
    logs -f
```

## Troubleshooting

### Images Not Found After Pull

If images aren't found after pulling:
```bash
# Verify images were pulled
docker images | grep stg_rd

# Make sure you tagged them correctly
docker tag your-username/stg_rd:frontend-$VERSION stg_rd-frontend:latest
docker tag your-username/stg_rd:backend-$VERSION stg_rd-backend:latest
```

### Port Conflicts

If ports are already in use:
```bash
# Check what's using the ports
sudo netstat -tulpn | grep -E "50001|50002|50003|63791"

# Or update ports in config/.env.production
```

### Database Connection Issues

```bash
# Verify ArangoDB is running
docker compose --env-file config/.env.production \
    -f deploy/docker-compose.yaml \
    -f deploy/docker-compose.prod.yml \
    -f deploy/docker-compose.stg_prod.yml \
    ps arangodb

# Check ArangoDB logs
docker compose --env-file config/.env.production \
    -f deploy/docker-compose.yaml \
    -f deploy/docker-compose.prod.yml \
    -f deploy/docker-compose.stg_prod.yml \
    logs arangodb
```

### Rollback (If Needed)

If something goes wrong, you can rollback:

```bash
# Stop current deployment
docker compose \
    --env-file config/.env.production \
    -f deploy/docker-compose.yaml \
    -f deploy/docker-compose.prod.yml \
    -f deploy/docker-compose.stg_prod.yml \
    down

# Pull previous version from Docker Hub
docker pull your-username/stg_rd:frontend-$PREVIOUS_VERSION
docker pull your-username/stg_rd:backend-$PREVIOUS_VERSION

# Tag and deploy
docker tag your-username/stg_rd:frontend-$PREVIOUS_VERSION stg_rd-frontend:latest
docker tag your-username/stg_rd:backend-$PREVIOUS_VERSION stg_rd-backend:latest

./scripts/deploy-tested-images.sh --version $PREVIOUS_VERSION --skip-backup
```

## Security Best Practices

1. **Use strong passwords** in `config/.env.production`
2. **Never commit** `.env.production` to version control
3. **Use SSH keys** instead of passwords for server access
4. **Set repository to Private** on Docker Hub
5. **Use Docker registry** for production (more secure than file transfer)
6. **Limit network access** - only expose necessary ports
7. **Monitor logs** after deployment for any issues

## Next Steps

- Set up automated backups
- Configure monitoring and alerting
- Set up CI/CD pipeline for automated deployments
- Document your production environment specifics
- Set up log aggregation (ELK, Loki, etc.)
