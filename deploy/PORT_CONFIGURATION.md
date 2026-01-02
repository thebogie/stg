# Docker Compose Port Configuration

This document outlines the standardized port configuration for all three environments.

## Port Summary

### e2e_env (E2E Testing)
- **Frontend**: `50023` (host) → `8080` (container)
- **Backend**: `50022` (host) → `50022` (container)
- **ArangoDB**: `50021` (host) → `8529` (container)
- **Redis**: `63790` (host) → `6379` (container)

**Network**: `e2e_env`  
**Compose Files**: `docker-compose.yaml` + `docker-compose.prod.yml` + `docker-compose.stg_prod.yml` + `docker-compose.e2e.yml`

### hybrid_dev_env (Development)
- **Frontend**: `50013` (host) → `50013` (container)
- **Backend**: `50012` (host) → `50012` (container)
- **ArangoDB**: `50011` (host) → `8529` (container)
- **Redis**: `6379` (host) → `6379` (container)

**Network**: `hybrid_dev_env`  
**Compose Files**: `docker-compose.yaml` + `docker-compose.dev.yml`  
**Env File**: `config/.env.development`

### stg_prod (Production/Staging)
- **Frontend**: `50001` (host) → `50001` (container)
- **Backend**: `50002` (host) → `50002` (container)
- **ArangoDB**: `50003` (host) → `8529` (container)
- **Redis**: `63791` (host) → `6379` (container)

**Network**: `stg_prod`  
**Compose Files**: `docker-compose.yaml` + `docker-compose.prod.yml` + `docker-compose.stg_prod.yml`  
**Env File**: `config/.env.production`

## Port Ranges

- **50001-50003**: stg_prod
- **50011-50013**: hybrid_dev_env
- **50021-50023**: e2e_env
- **6379**: hybrid_dev_env Redis (standard)
- **63790**: e2e_env Redis
- **63791**: stg_prod Redis

## Container Internal Ports

All containers use standard internal ports for inter-container communication:
- **Frontend (nginx)**: Listens on `FRONTEND_PORT` environment variable (typically 8080 or 50003)
- **Backend**: Listens on `SERVER_PORT` environment variable
- **ArangoDB**: Always `8529` (internal)
- **Redis**: Always `6379` (internal)

## Usage

### Start e2e_env
```bash
just test-frontend-e2e
# or
./scripts/start-e2e-docker.sh
```

### Start hybrid_dev_env
```bash
./deploy/deploy.sh --env development --build
```

### Start stg_prod
```bash
./deploy/deploy.sh --env stg_prod --build
# or
./deploy/deploy.sh --env production --build
```

## Notes

- All environments can run simultaneously (different ports)
- Each environment uses an isolated Docker network
- Container-to-container communication uses internal ports (8529, 6379)
- Host-to-container communication uses the mapped ports above

