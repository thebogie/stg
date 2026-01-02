# Production Deployment Guide

## Production Setup

The production setup uses the unified `deploy/` directory:
- **Environment file**: `config/.env.production` (in repo, gitignored)
- **Deployment script**: `deploy/deploy.sh` (unified for dev and prod)
- **Production override**: `deploy/docker-compose.prod.yml`
- **Compose files**: Base from repo, override from production directory

## Industry-Standard Improvements

### Unified Deployment

Use the `deploy/deploy.sh` script which supports both dev and prod:

```bash
# On production server
cd /path/to/stg_rd

# Setup production environment (first time)
./config/setup-env.sh production
# Edit config/.env.production with production values

# Deploy
./deploy/deploy.sh --env production --build
```

**Benefits:**
- Single script for all environments
- No hardcoded paths
- Works from anywhere
- Better error handling
- Industry-standard deployment process

## Recommended Production Workflow

### 1. Environment Setup

```bash
# On production server
cd /home/thebogie/stg/stg_rd

# Create production environment
./config/setup-env.sh production

# Edit with production values
nano config/.env.production
```

### 2. Deployment Process

```bash
# Pull latest code
git pull origin main

# Build and deploy
./deploy/deploy.sh --env production --build

# Verify deployment
./deploy/deploy.sh --env production --status
./deploy/deploy.sh --env production --logs
```

### 3. Rollback (if needed)

```bash
# Stop current deployment
./deploy/deploy.sh --env production --down

# Checkout previous version
git checkout <previous-commit>

# Redeploy
./deploy/deploy.sh --env production --build
```

## Industry Best Practices

### 1. Health Checks
Already configured in docker-compose.yaml ✓

### 2. Graceful Shutdowns
Add to docker-compose.prod.yml:
```yaml
stop_grace_period: 30s
```

### 3. Resource Limits
Add to docker-compose.prod.yml:
```yaml
deploy:
  resources:
    limits:
      cpus: '2'
      memory: 2G
    reservations:
      cpus: '1'
      memory: 1G
```

### 4. Restart Policies
Already set to `always` in prod override ✓

### 5. Secrets Management
- Keep `.env.production` outside repo (gitignored)
- Use environment-specific secrets
- Consider using Docker secrets or external secret managers for sensitive data

### 6. Monitoring & Logging
- Use `--logs` to view logs
- Consider adding log aggregation (ELK, Loki, etc.)
- Set up health check monitoring

### 7. Backup Strategy
Your ArangoDB backup script is good! Consider:
- Automated backups before deployments
- Backup verification
- Point-in-time recovery

## Migration Path

### Migrate to Unified Script

1. Copy your `.env.production` to `config/.env.production`
2. Use `deploy/deploy.sh --env production --build` for deployments
3. Test thoroughly
4. Update any automation/CI/CD
5. Remove old `_prod/` directory (if it exists)

### Phase 3: Add CI/CD (Future)

- GitHub Actions / GitLab CI
- Automated testing before deployment
- Automated rollback on failure
- Blue-green deployments

