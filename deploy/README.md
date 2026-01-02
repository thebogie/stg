# Deployment Directory

This directory contains all deployment configuration and scripts for both **development** and **production** environments.

## Quick Start

### Development
```bash
./deploy/deploy.sh --env development --build
```

### Production
```bash
./deploy/deploy.sh --env production --build
```

## Files

### Docker Compose Files
- **`docker-compose.yaml`** - Base configuration (shared by dev and prod)
- **`docker-compose.dev.yml`** - Development overrides (volume mounts, dev settings)
- **`docker-compose.prod.yml`** - Production overrides (no volumes, prod settings, resource limits)

### Deployment Scripts
- **`deploy.sh`** - Unified deployment script (handles both dev and prod)
  - Usage: `./deploy/deploy.sh --env [development|production] [options]`
  - Options: `--build`, `--down`, `--logs`, `--status`, `--restart`, `--clean`
  
- **`deploy-prod.sh`** - Advanced production deployment script
  - Includes backup, rollback, and verification
  - Usage: `./deploy/deploy-prod.sh [--rollback]`

### Documentation
- **`PRODUCTION_DEPLOYMENT.md`** - Production deployment guide
- **`INDUSTRY_STANDARDS.md`** - Industry best practices

## Environment Files

Environment files are stored in `config/` directory:
- `config/.env.development` - Development environment (gitignored)
- `config/.env.production` - Production environment (gitignored)
- `config/env.*.template` - Templates (safe to commit)

Create them with:
```bash
./config/setup-env.sh development
./config/setup-env.sh production
```

## Common Commands

### Development
```bash
# Start
./deploy/deploy.sh --env development --build

# View logs
./deploy/deploy.sh --env development --logs

# Stop
./deploy/deploy.sh --env development --down

# Status
./deploy/deploy.sh --env development --status
```

### Production
```bash
# Deploy
./deploy/deploy.sh --env production --build

# View logs
./deploy/deploy.sh --env production --logs

# Status
./deploy/deploy.sh --env production --status

# Advanced deployment (with backup/rollback)
./deploy/deploy-prod.sh
```

## Migration from Old Structure

If you were using `_prod/stg.sh`:
1. Copy your `.env.production` to `config/.env.production`
2. Use `./deploy/deploy.sh --env production --build` instead
3. The new script works the same way but is unified for all environments

## Benefits

- ✅ **Unified**: One script for all environments
- ✅ **No hardcoded paths**: Works from anywhere
- ✅ **Industry standard**: Follows best practices
- ✅ **Better error handling**: Proper validation and rollback
- ✅ **Easy to use**: Simple command-line interface



