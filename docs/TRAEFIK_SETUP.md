# Frontend with External Traefik

## Overview

If you have **Traefik already configured externally** (common with Proxmox setups) that routes SSL traffic to your frontend container on port 50003, the frontend container is simplified:

- ✅ **No API proxy needed** - Traefik routes `/api/` directly to backend
- ✅ **No SSL/TLS** - Traefik handles SSL termination
- ✅ **Simpler config** - Just static files, SPA routing, and WASM support

## Simplified Frontend Container

### Simplified Frontend Container

The main `Dockerfile.frontend` has been updated to:
- ✅ Serves static files
- ✅ Handles SPA routing (`try_files` for Yew router)
- ✅ Sets correct WASM MIME types
- ✅ Basic caching headers
- ❌ **No API proxy** (External Traefik handles this)
- ❌ **Minimal security headers** (External Traefik handles SSL/headers)

**Benefits:**
- Smaller image (simpler config)
- Less duplication (Traefik handles routing)
- Easier to maintain

### Option 2: Keep Current Nginx (If Needed)

Keep the current `Dockerfile.frontend` if:
- You want container-level API proxy as fallback
- You need container-level security headers
- You're not using Traefik for all routing

## Traefik Configuration

### Example Traefik Labels for Frontend

Add these labels to your `docker-compose.yaml` or Traefik config:

```yaml
services:
  frontend:
    labels:
      # Traefik routing
      - "traefik.enable=true"
      - "traefik.http.routers.frontend.rule=Host(`yourdomain.com`)"
      - "traefik.http.routers.frontend.entrypoints=websecure"
      - "traefik.http.routers.frontend.tls.certresolver=letsencrypt"
      
      # Service definition
      - "traefik.http.services.frontend.loadbalancer.server.port=8080"
      
      # API routing (if Traefik handles it)
      - "traefik.http.routers.api.rule=Host(`yourdomain.com`) && PathPrefix(`/api/`)"
      - "traefik.http.routers.api.entrypoints=websecure"
      - "traefik.http.routers.api.tls.certresolver=letsencrypt"
      - "traefik.http.services.api.loadbalancer.server.port=50002"
      
      # Security headers middleware (optional)
      - "traefik.http.middlewares.security-headers.headers.customRequestHeaders.X-Forwarded-Proto=https"
      - "traefik.http.middlewares.security-headers.headers.customResponseHeaders.X-Content-Type-Options=nosniff"
      - "traefik.http.middlewares.security-headers.headers.customResponseHeaders.X-Frame-Options=DENY"
      - "traefik.http.routers.frontend.middlewares=security-headers"
```

### Docker Compose (Standard)

Since Traefik is configured externally, use the standard `docker-compose.yaml`:

```yaml
services:
  frontend:
    container_name: frontend
    image: ${FRONTEND_IMAGE:-stg_rd-frontend:${IMAGE_TAG:-latest}}
    build:
      context: ..
      dockerfile: ./frontend/Dockerfile.frontend  # Standard Dockerfile (simplified)
    ports:
      - "${FRONTEND_PORT}:${FRONTEND_PORT}"  # Traefik routes to this port
    restart: unless-stopped
    # No Traefik labels needed - Traefik configured externally
```

## Setup

### 1. Standard Configuration

The main `Dockerfile.frontend` is already simplified - no changes needed!

### 2. Traefik Configuration (External)

Your external Traefik should be configured to:
- Route SSL traffic → frontend container on port **50003**
- Route `/api/*` → backend container on port **50002**

### 3. Docker Compose

Use the standard `docker-compose.yaml` - no special configuration needed:
```bash
docker compose -f deploy/docker-compose.yaml \
  --env-file config/.env.production \
  up -d
```

## What External Traefik Should Handle

### Routing Configuration

Your Traefik should be configured to route:
- `/` → `192.168.1.51:50003` (frontend container)
- `/api/*` → `192.168.1.51:50002` (backend container)

This means:
- ✅ Traefik handles SSL termination
- ✅ Traefik routes all `/` requests to frontend
- ✅ Traefik routes all `/api/*` requests to backend
- ✅ Frontend container only serves static files on port 50003
- ✅ Backend container serves API on port 50002

### SSL/TLS
- Let's Encrypt certificates
- HTTPS redirect
- TLS termination

### Security Headers (Optional)
- Content-Security-Policy
- X-Frame-Options
- HSTS
- X-Content-Type-Options
- etc.

### Load Balancing (If Multiple Instances)
- Round-robin or other strategies
- Health checks

## What Container Still Needs

### SPA Routing
- `try_files $uri $uri/ /index.html` for Yew router
- Handled by Nginx in container

### WASM MIME Types
- `Content-Type: application/wasm`
- Handled by Nginx in container

### Static File Caching
- Cache-Control headers for assets
- Handled by Nginx in container

## Comparison

| Feature | Current (Full Nginx) | Simplified (Traefik) |
|---------|---------------------|----------------------|
| Static file serving | ✅ Nginx | ✅ Nginx |
| SPA routing | ✅ Nginx | ✅ Nginx |
| API proxy | ✅ Nginx | ❌ Traefik |
| SSL/TLS | ❌ (HTTP only) | ✅ Traefik |
| Security headers | ✅ Nginx | ✅ Traefik |
| Load balancing | ❌ | ✅ Traefik |
| Image size | Larger | Smaller |
| Complexity | Higher | Lower |

## Current Setup

The main `Dockerfile.frontend` is already configured for external Traefik:
- ✅ No API proxy (Traefik handles routing)
- ✅ Minimal security headers (Traefik handles SSL/headers)
- ✅ Simple static file serving
- ✅ SPA routing support
- ✅ WASM MIME types

## Testing

```bash
# Build and test
docker compose -f deploy/docker-compose.yaml \
  --env-file config/.env.production \
  build frontend

# Start frontend
docker compose -f deploy/docker-compose.yaml \
  --env-file config/.env.production \
  up -d frontend

# Check it's serving on port 50003
curl http://localhost:50003

# Check WASM MIME type
curl -I http://localhost:50003/frontend_bg.wasm | grep Content-Type
```

## Summary

- ✅ **Frontend container**: Simplified, no API proxy, serves static files on port 50003
- ✅ **Backend container**: Serves API on port 50002
- ✅ **External Traefik**: 
  - Routes `/` → `192.168.1.51:50003` (frontend)
  - Routes `/api/*` → `192.168.1.51:50002` (backend)
  - Handles SSL termination
- ✅ **No special config needed**: Use standard `docker-compose.yaml`
- ✅ **Deploy normally**: Use your standard deployment workflow

The frontend container is now optimized for this setup - it just serves static files and handles SPA routing, while Traefik handles all the reverse proxy and routing logic.

