# Deploy Tested Docker Containers to Production

This guide walks you through deploying your tested Docker containers from your development machine to your production server.

## Prerequisites

- ✅ All tests have passed on your development machine
- ✅ Docker images have been built and tested
- ✅ Production server has Docker and Docker Compose installed
- ✅ SSH access to production server
- ✅ Production server has the project repository cloned

## Step-by-Step Deployment Process

### Step 1: Export Tested Images (On Development Machine)

First, export your tested Docker images to tar.gz files that can be transferred to production:

```bash
# From your project root directory
cd /home/thebogie/work/stg

# Export the tested images
./scripts/export-tested-images.sh
```

**What this does:**
- Exports frontend and backend images to `_build/artifacts/`
- Creates checksums for verification
- Generates deployment instructions

**Output files:**
- `_build/artifacts/frontend-v<version>.tar.gz`
- `_build/artifacts/backend-v<version>.tar.gz`
- `_build/artifacts/*.sha256` (checksums)
- `_build/artifacts/deploy-info-<version>.txt`

**Note the version tag** - you'll need it for deployment. It will be displayed in the output.

**Important:** If you ran E2E tests (`just test-frontend-e2e`), the script will automatically detect those images and create a version file. However, if you want proper version tracking with git commit info, you can run:

```bash
# This will be fast if images already exist (uses Docker cache)
./scripts/build-prod-images.sh
```

Then run the export script again. The `build-prod-images.sh` script creates proper version tags and tracking, but if images already exist from testing, it will reuse them quickly.

### Step 2: Transfer Images to Production Server

Transfer the exported image files to your production server:

```bash
# From your development machine
# Replace USER and PRODUCTION_SERVER with your actual values
scp _build/artifacts/*.tar.gz* user@production-server:/tmp/

# Example:
# scp _build/artifacts/*.tar.gz* deploy@prod.example.com:/tmp/
```

**Alternative: Using rsync (better for large files):**
```bash
rsync -avz --progress _build/artifacts/*.tar.gz* user@production-server:/tmp/
```

**Alternative: Using Docker Registry (Recommended for frequent deployments):**

If you have a Docker registry (Docker Hub, AWS ECR, etc.):

```bash
# Tag images
docker tag stg_rd-frontend:tested your-registry/stg_rd-frontend:v<version>
docker tag stg_rd-backend:tested your-registry/stg_rd-backend:v<version>

# Push to registry
docker push your-registry/stg_rd-frontend:v<version>
docker push your-registry/stg_rd-backend:v<version>
```

Then on production server:
```bash
docker pull your-registry/stg_rd-frontend:v<version>
docker pull your-registry/stg_rd-backend:v<version>
```

### Step 3: Prepare Production Server

**On your production server:**

1. **Navigate to project directory:**
   ```bash
   cd /path/to/stg  # Replace with your actual production path
   ```

2. **Ensure production environment is configured:**
   ```bash
   # If not already done, create production environment file
   ./config/setup-env.sh production
   
   # Edit with your production values
   nano config/.env.production
   ```

   **Important settings to verify:**
   - `VOLUME_PATH` - Absolute path for persistent data (e.g., `/var/lib/stg_rd`)
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

4. **Create volume directories (if needed):**
   ```bash
   # Check VOLUME_PATH in your .env.production file first
   sudo mkdir -p /var/lib/stg_rd/arango_data
   sudo mkdir -p /var/lib/stg_rd/arango_apps_data
   sudo chown -R $USER:$USER /var/lib/stg_rd  # Adjust permissions as needed
   ```

### Step 4: Deploy Images on Production Server

**On your production server:**

1. **Load the Docker images:**
   ```bash
   # Load frontend image
   gunzip -c /tmp/frontend-v<version>.tar.gz | docker load
   
   # Load backend image
   gunzip -c /tmp/backend-v<version>.tar.gz | docker load
   
   # Verify images are loaded
   docker images | grep -E "frontend|backend"
   ```

2. **Deploy using the deployment script:**
   ```bash
   # Get the version tag from the export (or use 'tested' if images were tagged that way)
   # The script can auto-detect the version from the tar.gz files
   
   ./scripts/deploy-tested-images.sh --version v<version> --image-dir /tmp
   
   # Or let it auto-detect:
   ./scripts/deploy-tested-images.sh --image-dir /tmp
   ```

   **What this script does:**
   - Verifies checksums
   - Loads images (if not already loaded)
   - Creates database backup (optional, can skip with `--skip-backup`)
   - Stops old containers
   - Starts new containers with tested images
   - Runs migrations (optional, can skip with `--skip-migrations`)
   - Verifies deployment health

3. **Alternative: Manual deployment (if you prefer more control):**
   ```bash
   # Set environment variables
   export ENV_FILE=config/.env.production
   export IMAGE_TAG=v<version>  # or 'tested'
   
   # Stop existing containers
   docker compose \
       --env-file config/.env.production \
       -f deploy/docker-compose.yaml \
       -f deploy/docker-compose.prod.yml \
       -f deploy/docker-compose.stg_prod.yml \
       down
   
   # Start new containers
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
```

**Check service health:**
```bash
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

3. **Clean up transferred files (optional):**
   ```bash
   # Remove the tar.gz files from /tmp
   rm /tmp/*.tar.gz*
   ```

## Troubleshooting

### Images Not Found

If deployment script can't find images:
```bash
# Check what images are available
docker images | grep -E "frontend|backend|stg_rd"

# If images have 'tested' tag instead of version tag:
export IMAGE_TAG=tested
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

# Load previous version images (if you have them)
gunzip -c /tmp/frontend-v<previous-version>.tar.gz | docker load
gunzip -c /tmp/backend-v<previous-version>.tar.gz | docker load

# Deploy previous version
./scripts/deploy-tested-images.sh --version v<previous-version> --image-dir /tmp
```

## Quick Reference

### Development Machine Commands
```bash
# Export tested images
./scripts/export-tested-images.sh

# Transfer to production
scp _build/artifacts/*.tar.gz* user@production-server:/tmp/
```

### Production Server Commands
```bash
# Load and deploy
./scripts/deploy-tested-images.sh --image-dir /tmp

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

## Security Best Practices

1. **Use strong passwords** in `config/.env.production`
2. **Never commit** `.env.production` to version control
3. **Use SSH keys** instead of passwords for server access
4. **Verify checksums** before deploying (script does this automatically)
5. **Backup database** before deployment (script does this automatically)
6. **Use Docker registry** for production (more secure than SCP)
7. **Limit network access** - only expose necessary ports
8. **Monitor logs** after deployment for any issues

## Next Steps

- Set up automated backups
- Configure monitoring and alerting
- Set up CI/CD pipeline for automated deployments
- Document your production environment specifics
- Set up log aggregation (ELK, Loki, etc.)

