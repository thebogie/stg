# Hybrid Development - Quick Start

**Best setup for active development with full debugger support!**

## One-Command Setup

```bash
./scripts/setup-hybrid-dev.sh
```

This will:
1. Start ArangoDB and Redis in Docker (or use existing containers)
2. Load production data if available (or skip if not found)
3. Show you next steps

**Skip data load:**
```bash
./scripts/setup-hybrid-dev.sh --skip-data
```

## Complete Workflow

### 1. Setup Environment (One Time)
```bash
./scripts/setup-hybrid-dev.sh
```

This handles everything:
- ✅ Starts dependencies (ArangoDB + Redis)
- ✅ Loads production data (if available)
- ✅ Ready for development

### 2. Start Backend with Debugger

**In VSCode:**
1. Open Debug panel (F5 or click Debug icon)
2. Select **"Debug Backend (Hybrid Dev)"** from dropdown
3. Press **F5** to start
4. Set breakpoints anywhere in your Rust code
5. Backend connects to Docker services automatically

**Or in Terminal:**
```bash
# Load environment variables (development by default)
source scripts/load-env.sh

# Or explicitly specify environment
source scripts/load-env.sh development  # or 'production'

# Run backend (uses variables from .env file)
export RUST_LOG=debug
cargo run --package backend --bin backend
```

### 3. Start Frontend

**Option A: VSCode Task**
1. Press `Ctrl+Shift+P` (or `Cmd+Shift+P`)
2. Type "Tasks: Run Task"
3. Select **"frontend: trunk serve"**

**Option B: Script**
```bash
./scripts/start-frontend.sh
```

**Option C: Terminal**
```bash
cd frontend
npm run build:css:prod  # One time
trunk serve --no-default-features --features frontend
```

Frontend available at: **http://localhost:${FRONTEND_PORT}** (port from `config/.env.development`)

## Debugging

### Backend Debugging
- ✅ Set breakpoints in any `.rs` file
- ✅ Step through code (F10 = step over, F11 = step into)
- ✅ Inspect variables in Variables panel
- ✅ View logs in Debug Console
- ✅ Watch expressions

### Frontend Debugging
- Use browser DevTools (F12)
- WASM debugging in browser debugger
- Network tab for API calls
- Console for logs

## Environment Variables

Your `config/.env.development` (or `config/.env.production`) should have:
```bash
# Ports (used by all scripts)
ARANGODB_PORT=50011
BACKEND_PORT=50012
FRONTEND_PORT=50013
REDIS_PORT=6379

# URLs (automatically constructed from ports)
ARANGO_URL=http://localhost:${ARANGODB_PORT}
REDIS_URL=redis://localhost:${REDIS_PORT}/
SERVER_PORT=${BACKEND_PORT}
BACKEND_URL=http://localhost:${BACKEND_PORT}
```

**Note**: All scripts use `scripts/load-env.sh` to load these variables automatically. The same scripts work for both development and production by specifying the environment.

## Daily Workflow

**First time setup:**
```bash
./scripts/setup-hybrid-dev.sh
```

**Daily development:**
1. Start backend debugger (F5) - containers already running
2. Start frontend (Ctrl+Shift+P → "frontend: trunk serve")
3. Develop!

**Stop everything:**
1. Stop backend: Stop debugger in VSCode (Shift+F5)
2. Stop frontend: Stop task or Ctrl+C in terminal
3. Stop dependencies (optional - they'll restart automatically):
   ```bash
   # Uses environment from RUST_ENV or defaults to development
   source scripts/load-env.sh
   docker compose -p hybrid_dev_env -f deploy/docker-compose.deps.yml --env-file config/.env.${RUST_ENV} down
   ```

## Benefits

✅ **Fast iteration** - No Docker rebuilds needed  
✅ **Full debugger** - Breakpoints, step-through, variable inspection  
✅ **Hot reload** - Frontend auto-reloads on changes  
✅ **Production data** - Real data in ArangoDB  
✅ **Best DX** - Best development experience  

## Troubleshooting

**Backend can't connect:**
```bash
# Load environment to get correct ports
source scripts/load-env.sh

# Check services are running
docker ps | grep -E "arangodb-dev|redis-dev"

# Test connections (uses ports from .env file)
curl http://localhost:${ARANGODB_PORT}/_api/version
redis-cli -h localhost -p ${REDIS_PORT} ping
```

**Port conflicts:**
```bash
# Load environment to get correct ports
source scripts/load-env.sh

# Check what's using ports
lsof -i :${ARANGODB_PORT}  # ArangoDB
lsof -i :${REDIS_PORT}      # Redis
lsof -i :${BACKEND_PORT}    # Backend
lsof -i :${FRONTEND_PORT}   # Frontend
```

## Next Steps: Production Release

After developing in hybrid mode, follow the [CI/CD Workflow](docs/CI_CD_WORKFLOW.md) to:
1. Build production images
2. Test production containers
3. Deploy to production

## Full Documentation

- [Hybrid Development Guide](docs/setup/HYBRID_DEVELOPMENT.md) - Complete hybrid dev details
- [CI/CD Workflow](docs/CI_CD_WORKFLOW.md) - Complete workflow from dev to production
- [Test-Then-Deploy Guide](docs/TEST_THEN_DEPLOY_WORKFLOW.md) - Production deployment



