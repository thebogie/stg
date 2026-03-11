# Frontend in Production

## Overview

In production, the frontend is **not started manually** like in development. Instead, it's:

1. **Built** into a Docker image (with Trunk, optimized WASM, and Nginx)
2. **Deployed** as part of the Docker Compose stack
3. **Served** by Nginx (not Trunk dev server)

**Note**: If you're using **Traefik** as your reverse proxy (common with Proxmox), see [Traefik Setup Guide](TRAEFIK_SETUP.md) for a simplified configuration.

## Production Architecture

### Development vs Production

| Aspect | Development | Production |
|--------|-------------|------------|
| **Server** | Trunk dev server | Nginx |
| **Build** | On-the-fly | Pre-built in Docker image |
| **WASM** | Debug build | Optimized release build |
| **Hot Reload** | ✅ Yes | ❌ No |
| **Start Command** | `./scripts/start-frontend.sh` | Docker container |

### Production Build Process

The frontend Docker image (`Dockerfile.frontend`) does:

1. **Installs dependencies** (Rust, Node.js, Trunk, Binaryen)
2. **Builds CSS** (`npm run build:css:prod`)
3. **Builds WASM** (`trunk build --release`)
4. **Optimizes WASM** (`wasm-opt` for size/speed)
5. **Serves with Nginx** (static files + API proxy)

## Production Deployment Workflow

### Step 1: Build Production Images

```bash
./scripts/build-prod-images.sh
```

**What it does:**
- Builds frontend Docker image with production optimizations
- Builds backend Docker image
- Tags images with version (git commit + timestamp)
- Creates `_build/.build-version` file

**Frontend build includes:**
- ✅ Minified CSS
- ✅ Optimized WASM (wasm-opt)
- ✅ Nginx configuration
- ✅ Security headers
- ✅ API proxy setup

### Step 2: Test Production Containers Locally

```bash
./scripts/test-prod-containers.sh --use-test-db
```

This starts the **production containers** (including frontend) locally so you can test them before deploying.

### Step 3: Export Tested Images

```bash
./scripts/export-tested-images.sh
```

Exports the tested Docker images (including frontend) to `_build/artifacts/` as tar.gz files.

### Step 4: Deploy to Production

**On production server:**

```bash
# 1. Transfer images
scp _build/artifacts/*.tar.gz* user@production-server:/tmp/

# 2. Deploy (on production server)
cd /path/to/stg_rd
./scripts/deploy-tested-images.sh --version v<version> --image-dir /tmp
```

**What happens:**
- Loads frontend Docker image from tar.gz
- Starts frontend container with Nginx
- Frontend serves static files from `/usr/share/nginx/html`
- Nginx proxies `/api/` requests to backend
- Health checks verify frontend is running

## Production Frontend Container

### Container Details

- **Image**: `stg_rd-frontend:v<version>`
- **Base**: Nginx Alpine
- **Port**: `50003` (configurable via `FRONTEND_PORT`)
- **Command**: `nginx -g "daemon off;"`
- **Health Check**: `curl -f http://localhost:50003`

### Nginx Configuration

The frontend container runs Nginx with:

- **Static file serving** for built frontend
- **SPA routing** (all routes → `index.html`)
- **API proxy** (`/api/` → backend container)
- **Security headers** (CSP, HSTS, etc.)
- **WASM support** (correct MIME types, CORS headers)
- **Caching** (1 year for static assets)

### Environment Variables

Set in `config/.env.production`:

```bash
FRONTEND_PORT=50003
BACKEND_URL=http://backend:50002
HTTPS_ENABLED=false  # Set to true if using HTTPS
```

## Complete Production Workflow

### Full Deployment (from Development)

```bash
# === On Development Machine ===

# 1. Build production images
./scripts/build-prod-images.sh

# 2. Test locally
./scripts/test-prod-containers.sh --use-test-db
# Test frontend at http://localhost:50013

# 3. Export tested images
./scripts/export-tested-images.sh

# 4. Transfer to production
scp _build/artifacts/*.tar.gz* user@production:/tmp/

# === On Production Server ===

# 5. Backup database
./scripts/backup-prod-db.sh

# 6. Deploy tested images
./scripts/deploy-tested-images.sh --version v<version> --image-dir /tmp

# 7. Verify
curl http://localhost:50003  # Frontend
curl http://localhost:50002/health  # Backend
```

### Quick Deployment (if images already built)

```bash
# On production server
./scripts/deploy-tested-images.sh --version v<version> --image-dir /tmp
```

## Managing Production Frontend

### Check Status

```bash
docker ps | grep frontend
docker logs frontend
```

### View Logs

```bash
# All logs
docker logs frontend

# Follow logs
docker logs -f frontend

# Last 100 lines
docker logs --tail 100 frontend
```

### Restart Frontend

```bash
# Restart just frontend
docker restart frontend

# Or restart entire stack
docker compose --env-file config/.env.production \
  -f deploy/docker-compose.yaml \
  -f deploy/docker-compose.prod.yml \
  restart frontend
```

### Update Frontend Only

```bash
# 1. Build new frontend image
./scripts/build-prod-images.sh

# 2. Export frontend image
./scripts/export-tested-images.sh

# 3. Transfer to production
scp _build/artifacts/frontend-*.tar.gz* user@production:/tmp/

# 4. On production: Load and restart
docker load < /tmp/frontend-v<version>.tar.gz
docker compose --env-file config/.env.production \
  -f deploy/docker-compose.yaml \
  -f deploy/docker-compose.prod.yml \
  up -d frontend
```

## Troubleshooting

### Frontend Not Loading

```bash
# Check container is running
docker ps | grep frontend

# Check logs
docker logs frontend

# Check port is accessible
curl http://localhost:50003

# Check Nginx config
docker exec frontend cat /etc/nginx/conf.d/default.conf
```

### API Requests Failing

```bash
# Check backend is running
docker ps | grep backend

# Check backend URL in frontend container
docker exec frontend env | grep BACKEND_URL

# Test backend directly
curl http://localhost:50002/health
```

### WASM Not Loading

```bash
# Check WASM files exist
docker exec frontend ls -la /usr/share/nginx/html/*.wasm

# Check Nginx MIME types
docker exec frontend cat /etc/nginx/mime.types | grep wasm

# Check browser console for CORS errors
```

### Rebuild Frontend

```bash
# Force rebuild (no cache)
docker compose --env-file config/.env.production \
  -f deploy/docker-compose.yaml \
  -f deploy/docker-compose.prod.yml \
  build --no-cache frontend
```

## Key Differences Summary

### Development
- ✅ `./scripts/start-frontend.sh` - Starts Trunk dev server
- ✅ Hot reload enabled
- ✅ Debug WASM build
- ✅ Runs on your machine

### Production
- ✅ Built into Docker image
- ✅ Served by Nginx
- ✅ Optimized WASM build
- ✅ Runs in Docker container
- ✅ Part of Docker Compose stack

## Using Traefik?

If you're using **Traefik** (common with Proxmox setups), you can simplify the frontend container:

- ✅ **Simplified Dockerfile**: `Dockerfile.frontend.simple` (no API proxy, Traefik handles routing)
- ✅ **Smaller image**: Less complexity, faster builds
- ✅ **Traefik handles**: SSL, reverse proxy, API routing, security headers

See [Traefik Setup Guide](TRAEFIK_SETUP.md) for complete instructions.

## Next Steps

- See [CI/CD Workflow](CI_CD_WORKFLOW.md) for complete workflow
- See [Traefik Setup](TRAEFIK_SETUP.md) if using Traefik reverse proxy
- See [Production Deployment](deploy/PRODUCTION_DEPLOYMENT.md) for server setup
- See [Test-Then-Deploy Workflow](TEST_THEN_DEPLOY_WORKFLOW.md) for deployment process

