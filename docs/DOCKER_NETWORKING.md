# Docker Networking for Production

## Overview

Understanding Docker networking is crucial for production deployment. This guide explains how different components communicate.

## Communication Flows

### 1. Browser → Frontend → Backend

```
User's Browser
    ↓ (HTTPS)
https://smacktalkgaming.com or http://192.168.1.51:50001
    ↓ (Nginx serves static files)
Frontend Container (port 50001)
    ↓ (API proxy)
Backend Container (port 50002)
```

**Key Points:**
- Frontend uses **relative URLs** (`/api/...`) 
- Nginx or Traefik proxies `/api/` requests to backend container
- Backend URL is handled by reverse proxy, NOT by frontend code

### 2. Backend → ArangoDB (Internal)

```
Backend Container
    ↓ (Docker internal network)
http://arangodb:8529
    ↓
ArangoDB Container (internal port 8529)
```

**Configuration:**
```bash
# In .env.production - this is for BACKEND container to use
ARANGO_URL=http://arangodb:8529  # ← Service name, NOT IP or domain!
```

**Why `arangodb:8529`?**
- `arangodb` is the **service name** from docker-compose.yaml
- Docker's internal DNS resolves `arangodb` to the container's IP
- `8529` is the **internal container port** (always 8529, not the external 50003)

### 3. Backend → Redis (Internal)

```
Backend Container
    ↓ (Docker internal network)
redis://redis:6379/
    ↓
Redis Container (internal port 6379)
```

**Configuration:**
```bash
# In .env.production - this is for BACKEND container to use
REDIS_URL=redis://redis:6379/  # ← Service name, NOT IP or domain!
```

## Port Mapping Explained

### External Ports (Host Machine)
These are what you access from outside Docker:

```yaml
ports:
  - "50001:50001"  # Frontend (host:container)
  - "50002:50002"  # Backend
  - "50003:8529"   # ArangoDB (host 50003 → container 8529)
  - "63791:6379"   # Redis (host 63791 → container 6379)
```

### Internal Ports (Container-to-Container)
These are used for service-to-service communication:

```yaml
# Backend environment (inside container)
ARANGO_URL=http://arangodb:8529   # ← Always 8529 (internal)
REDIS_URL=redis://redis:6379/     # ← Always 6379 (internal)
```

## Common Mistakes

### ❌ WRONG: Using External IPs
```bash
ARANGO_URL=http://192.168.1.51:50003  # ❌ This won't work inside container!
```

### ❌ WRONG: Using External Domain
```bash
ARANGO_URL=https://smacktalkgaming.com:50003  # ❌ This won't work!
```

### ✅ CORRECT: Using Service Names
```bash
ARANGO_URL=http://arangodb:8529  # ✅ Docker resolves this automatically
```

## How Docker Networking Works

1. **Docker creates an internal network** (`stg_prod` in your case)
2. **Each service gets a service name** (from docker-compose.yaml)
3. **Docker provides internal DNS** that resolves service names to container IPs
4. **Containers communicate using service names**, not IPs

## Example: Your Production Setup

```yaml
# docker-compose.yaml
services:
  backend:
    container_name: backend
    # Environment variable set by docker-compose
    environment:
      - ARANGO_URL=http://arangodb:8529  # ← Overrides .env.production
      
  arangodb:
    container_name: arangodb
    # Internal port is always 8529
    # External port is 50003 (from docker-compose.stg_prod.yml)
```

## Verifying Configuration

### Check what backend sees:
```bash
docker exec backend env | grep ARANGO_URL
# Should show: ARANGO_URL=http://arangodb:8529
```

### Test connectivity from backend:
```bash
docker exec backend curl http://arangodb:8529/_api/version
# Should return ArangoDB version JSON
```

### Check network:
```bash
docker network inspect stg_prod
# Shows all containers on the network and their IPs
```

## Summary

| Communication | From | To | URL Format |
|--------------|------|-----|------------|
| Browser → Frontend | Browser | Frontend Container | `https://smacktalkgaming.com` or `http://192.168.1.51:50001` |
| Frontend → Backend | Frontend Container | Backend Container | `/api/...` (relative, proxied by Nginx/Traefik) |
| Backend → ArangoDB | Backend Container | ArangoDB Container | `http://arangodb:8529` (service name) |
| Backend → Redis | Backend Container | Redis Container | `redis://redis:6379/` (service name) |
| External → ArangoDB | Your computer | ArangoDB Container | `http://192.168.1.51:50003` or `http://smacktalkgaming.com:50003` |

**Key Rule:** Inside containers, always use **service names** and **internal ports**. Outside containers (browser, your computer), use **external IPs/domains** and **external ports**.

