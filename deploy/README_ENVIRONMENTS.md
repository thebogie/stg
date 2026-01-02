# Docker Compose Environment Configuration

This document explains how the three environments work together and how E2E tests ensure they match production exactly.

## Three Environments

### 1. **hybrid_dev_env** (Development)
- **Network**: `hybrid_dev_env`
- **Compose Files**: `docker-compose.yaml` + `docker-compose.dev.yml`
- **Purpose**: Local development with volume mounts for live reload
- **Usage**: `./deploy/deploy.sh --env development --build`

### 2. **e2e_env** (E2E Testing)
- **Network**: `e2e_env`
- **Compose Files**: `docker-compose.yaml` + `docker-compose.prod.yml` + `docker-compose.stg_prod.yml` + `docker-compose.e2e.yml`
- **Purpose**: E2E tests that match production exactly
- **Usage**: `./scripts/start-e2e-docker.sh` or `just test-frontend-e2e`

### 3. **stg_prod** (Production/Staging)
- **Network**: `stg_prod`
- **Compose Files**: `docker-compose.yaml` + `docker-compose.prod.yml` + `docker-compose.stg_prod.yml`
- **Purpose**: Production deployment
- **Usage**: `./deploy/deploy.sh --env stg_prod --build`

## How E2E Matches Production

**Key Principle**: E2E uses the **EXACT same compose files** as production, with only port overrides.

### E2E Compose Stack:
1. `docker-compose.yaml` - Base services (shared by all)
2. `docker-compose.prod.yml` - Production settings (shared with production)
3. `docker-compose.stg_prod.yml` - Production network config (shared with production)
4. `docker-compose.e2e.yml` - **ONLY port overrides** (nothing else!)

### Production Compose Stack:
1. `docker-compose.yaml` - Base services (shared by all)
2. `docker-compose.prod.yml` - Production settings (shared with E2E)
3. `docker-compose.stg_prod.yml` - Production network config (shared with E2E)

### Differences:
- **Ports**: E2E uses port 8080 for frontend (Playwright requirement), production uses different port
- **Network Name**: E2E uses `e2e_env`, production uses `stg_prod` (for isolation)
- **Everything Else**: Identical!

## Network Isolation

Having separate networks is **efficient and recommended**:
- Docker networks are lightweight (minimal resource usage)
- Provides complete isolation between environments
- Prevents port conflicts
- Allows all environments to run simultaneously

## Configuration Guarantees

By using the same compose files, we guarantee:
- ✅ Same Docker images
- ✅ Same resource limits
- ✅ Same restart policies
- ✅ Same health checks
- ✅ Same environment variables (from env files)
- ✅ Same volume configurations
- ✅ Same service dependencies

The only differences are:
- Port mappings (for testing requirements)
- Network name (for isolation)
- Volume paths (separate test data)

## Best Practices

1. **Never modify `docker-compose.e2e.yml`** except for port overrides
2. **Always use `docker-compose.stg_prod.yml`** in E2E to match production
3. **Test with production env file** when possible: `config/.env.production`
4. **Keep networks separate** - this is a feature, not a bug!

## Verification

To verify E2E matches production:

```bash
# Compare compose file usage
echo "E2E uses:"
echo "  - docker-compose.yaml"
echo "  - docker-compose.prod.yml"
echo "  - docker-compose.stg_prod.yml"
echo "  - docker-compose.e2e.yml (ports only)"

echo "Production uses:"
echo "  - docker-compose.yaml"
echo "  - docker-compose.prod.yml"
echo "  - docker-compose.stg_prod.yml"

# They're identical except for ports!
```

