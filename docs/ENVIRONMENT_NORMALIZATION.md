# Environment Normalization Guide

All documentation and scripts have been normalized to work with both **development** and **production** environments using the same commands and workflows.

## Key Principle

**Same scripts, different environment files.** All scripts use `scripts/load-env.sh` to automatically load the correct environment configuration.

## Environment Selection

Scripts determine which environment to use in this order:

1. **Explicit argument**: `source scripts/load-env.sh production`
2. **RUST_ENV variable**: If `RUST_ENV` is set, it uses that environment
3. **Default**: Falls back to `development` if neither is specified

## Normalized Workflows

### Development Workflow

```bash
# 1. Setup environment (one time)
./config/setup-env.sh development
# Edit config/.env.development with your values

# 2. Start dependencies
./scripts/setup-hybrid-dev.sh
# Uses config/.env.development automatically

# 3. Start backend
# VSCode: Debug → "Debug Backend (Hybrid Dev)" → F5
# Or terminal: source scripts/load-env.sh && cargo run --package backend --bin backend

# 4. Start frontend
./scripts/start-frontend.sh
# Uses config/.env.development automatically
```

### Production Workflow

```bash
# 1. Setup environment (one time)
./config/setup-env.sh production
# Edit config/.env.production with your values

# 2. Start dependencies
RUST_ENV=production ./scripts/setup-hybrid-dev.sh
# Or: source scripts/load-env.sh production && ./scripts/setup-hybrid-dev.sh

# 3. Start backend
source scripts/load-env.sh production
cargo run --package backend --bin backend

# 4. Start frontend
source scripts/load-env.sh production
./scripts/start-frontend.sh
```

## Port Configuration

**No hardcoded ports!** All ports come from your environment file:

```bash
# In config/.env.development or config/.env.production
ARANGODB_PORT=50011
BACKEND_PORT=50012
FRONTEND_PORT=50013
REDIS_PORT=6379
```

Scripts automatically use these ports. To check your ports:

```bash
source scripts/load-env.sh  # or 'production'
echo "Frontend: http://localhost:${FRONTEND_PORT}"
echo "Backend: http://localhost:${BACKEND_PORT}"
echo "ArangoDB: http://localhost:${ARANGODB_PORT}"
```

## Environment Variables

All scripts use `scripts/load-env.sh` which:

1. Loads variables from `config/.env.development` or `config/.env.production`
2. Expands variable substitutions (e.g., `${ARANGODB_PORT}`)
3. Sets defaults if variables are missing
4. Exports `RUST_ENV` for other scripts
5. Constructs URLs from ports automatically

## Updated Documentation

All major documentation files have been updated:

- ✅ `HYBRID_DEV_QUICK_START.md` - Uses environment variables
- ✅ `docs/setup/HYBRID_DEVELOPMENT.md` - Normalized for both environments
- ✅ `docs/CI_CD_WORKFLOW.md` - Shows both dev and prod workflows
- ✅ `docs/DAILY_WORKFLOW.md` - Uses environment variables
- ✅ `README.md` - Shows normalized setup

## Key Changes

### Before (Hardcoded)
```bash
source config/.env.development
export ARANGO_URL=http://localhost:50001
export SERVER_PORT=50002
curl http://localhost:50003
```

### After (Normalized)
```bash
source scripts/load-env.sh  # or 'production'
# Variables automatically loaded from .env file
curl http://localhost:${FRONTEND_PORT}
```

## Benefits

✅ **Consistency**: Same commands work for dev and prod  
✅ **Flexibility**: Easy to switch environments  
✅ **Maintainability**: Port changes only in one place  
✅ **Documentation**: All docs show the same patterns  
✅ **No Hardcoding**: Everything comes from environment files  

## Quick Reference

| Task | Development | Production |
|------|-------------|------------|
| Load env | `source scripts/load-env.sh` | `source scripts/load-env.sh production` |
| Setup | `./scripts/setup-hybrid-dev.sh` | `RUST_ENV=production ./scripts/setup-hybrid-dev.sh` |
| Start frontend | `./scripts/start-frontend.sh` | `source scripts/load-env.sh production && ./scripts/start-frontend.sh` |
| Check ports | `source scripts/load-env.sh && echo $FRONTEND_PORT` | `source scripts/load-env.sh production && echo $FRONTEND_PORT` |

## See Also

- [Environment Usage Guide](../scripts/ENV_USAGE.md) - Detailed usage of `load-env.sh`
- [Hybrid Development Quick Start](../HYBRID_DEV_QUICK_START.md) - Fast development setup
- [CI/CD Workflow](CI_CD_WORKFLOW.md) - Complete workflow documentation

