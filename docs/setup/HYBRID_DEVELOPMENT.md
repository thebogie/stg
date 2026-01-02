# Hybrid Development Setup

This guide explains how to set up a hybrid development environment where:
- **ArangoDB** (with production data) runs in Docker
- **Redis** runs in Docker
- **Backend** runs locally in VSCode with debugger
- **Frontend** runs locally in VSCode with debugger

This setup gives you:
- ✅ Fast iteration (no Docker rebuilds)
- ✅ Full debugger support
- ✅ Production-like data for testing
- ✅ Hot reload for frontend
- ✅ Breakpoints and step-through debugging

## Prerequisites

1. **VSCode** with recommended extensions installed
2. **Docker** and Docker Compose
3. **Rust toolchain** installed locally
4. **Node.js** and npm (for frontend CSS building)
5. **Trunk** installed: `cargo install trunk`

## Quick Start

**One-command setup:**
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

## Setup Steps

### 1. Setup Environment (One Command)

**Recommended: Use the setup script**
```bash
./scripts/setup-hybrid-dev.sh
```

This single command:
- Starts ArangoDB and Redis in Docker (or uses existing containers)
- Loads production data if available (or skips if not found)
- Sets everything up for development

**Skip data load:**
```bash
./scripts/setup-hybrid-dev.sh --skip-data
```

**Manual setup (if needed):**
```bash
# Start dependencies only
./scripts/start-hybrid-dev.sh

# Load data separately
./scripts/load-prod-data.sh
```

**Using VSCode task:**
- Press Ctrl+Shift+P → "Tasks: Run Task" → "setup-hybrid-dev"

The script will:
- Look for data dumps in common locations (`_build/backups/`, `../_build/backups/`, etc.)
- Extract and restore the data to your local ArangoDB
- Create the database if it doesn't exist

**Data dump locations checked:**
- `./_build/backups/smacktalk.zip`
- `./_build/backups/smacktalk.tar`
- `../_build/backups/smacktalk.zip`
- `../_build/backups/smacktalk.tar`
- `./_build/dumps/dump.sanitized.json.gz`
- `./_build/dumps/dump.json`

**To export data from production:**
```bash
mkdir -p _build/dumps
cargo run --package scripts --bin export_prod_data \
  -- --arango-url http://prod-server:8529 \
     --arango-password <password> \
     --output _build/dumps/dump.json
```

### 3. Configure Environment

Create and configure your environment file:

**For Development:**
```bash
./config/setup-env.sh development
# Edit config/.env.development with your values
```

**For Production:**
```bash
./config/setup-env.sh production
# Edit config/.env.production with your values
```

**Environment file structure** (same for both dev and prod):
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

# Database credentials
ARANGO_DB=stg_rd
ARANGO_USERNAME=root
ARANGO_PASSWORD=your_password
ARANGO_ROOT_PASSWORD=your_password

# Server configuration
SERVER_HOST=0.0.0.0
```

**Note**: All scripts use `scripts/load-env.sh` to automatically load variables from the appropriate `.env` file. The same scripts work for both environments.

### 4. Start Backend with Debugger

**Option A: Using VSCode Debug Panel**

1. Open VSCode debug panel (F5 or Debug view)
2. Select "Debug Backend (Hybrid Dev)" from dropdown
3. Press F5 to start
4. Set breakpoints in your code
5. Backend will start and connect to Docker services

**Option B: Using Terminal**

```bash
# Load environment variables (development by default)
source scripts/load-env.sh

# Or explicitly specify environment
source scripts/load-env.sh development  # or 'production'

# Set logging (optional)
export RUST_LOG=debug
export RUST_BACKTRACE=1

# Run backend (uses variables from .env file)
cargo run --package backend --bin backend
```

### 5. Start Frontend

**Option A: Using VSCode Task**

1. Press `Ctrl+Shift+P` (or `Cmd+Shift+P` on Mac)
2. Type "Tasks: Run Task"
3. Select "frontend: trunk serve"
4. Frontend will build CSS and start Trunk dev server

**Option B: Using Script (Recommended)**

```bash
./scripts/start-frontend.sh
```

**Option C: Using Terminal**

```bash
cd frontend

# Build CSS first (one time)
npm run build:css:prod

# Start Trunk dev server (with hot reload)
trunk serve --no-default-features --features frontend
```

Frontend will be available at: http://localhost:${FRONTEND_PORT} (port from your `.env` file)

**Note:** For frontend debugging, use browser DevTools. The WASM code can be debugged in the browser's debugger.

### 6. Debug Both Together

1. **Start backend** with "Debug Backend (Hybrid Dev)" (F5)
2. **Start frontend** with task "frontend: trunk serve"
3. Both will run simultaneously with full debugging support

## VSCode Tasks

Available tasks (Ctrl+Shift+P → "Tasks: Run Task"):

- **start-dependencies** - Start ArangoDB and Redis in Docker
- **stop-dependencies** - Stop ArangoDB and Redis containers
- **load-prod-data** - Load production data into local ArangoDB

## Debugging Tips

### Backend Debugging

1. **Set breakpoints** in any Rust file
2. **Step through code** with F10 (step over), F11 (step into)
3. **Inspect variables** in the Variables panel
4. **Watch expressions** in the Watch panel
5. **View logs** in the Debug Console

### Frontend Debugging

1. **Set breakpoints** in Rust/WASM code
2. **Use browser DevTools** for JavaScript/WASM debugging
3. **Check Network tab** for API calls
4. **Use console.log** in Rust (via `gloo_console::log!`)

### Common Issues

**Backend can't connect to ArangoDB:**
```bash
# Load environment to get correct ports
source scripts/load-env.sh

# Check if ArangoDB is running
docker ps | grep arangodb-dev

# Check connection (uses port from .env file)
curl http://localhost:${ARANGODB_PORT}/_api/version

# Check environment variables
echo "ARANGO_URL: $ARANGO_URL"
echo "ARANGODB_PORT: $ARANGODB_PORT"
```

**Backend can't connect to Redis:**
```bash
# Load environment to get correct ports
source scripts/load-env.sh

# Check if Redis is running
docker ps | grep redis-dev

# Test connection (uses port from .env file)
redis-cli -h localhost -p ${REDIS_PORT} ping
```

**Port conflicts:**
```bash
# Load environment to get correct ports
source scripts/load-env.sh

# Check ports from your .env file
lsof -i :${ARANGODB_PORT}  # ArangoDB
lsof -i :${REDIS_PORT}      # Redis
lsof -i :${BACKEND_PORT}    # Backend
lsof -i :${FRONTEND_PORT}   # Frontend
```

**Data not loading:**
- Check ArangoDB logs: `docker logs arangodb-dev`
- Verify dump file exists and is valid
- Try manual restore: `arangorestore --server.endpoint tcp://localhost:50001 ...`

## Workflow

### Daily Development

**First time (one-time setup):**
```bash
./scripts/setup-hybrid-dev.sh
```

**Daily workflow:**
1. **Start backend in VSCode:**
   - Select "Debug Backend (Hybrid Dev)"
   - Press F5 (containers already running, no conflicts!)

2. **Start frontend:**
   - Run: `./scripts/start-frontend.sh`
   - Or VSCode: Ctrl+Shift+P → "Tasks: Run Task" → "frontend: trunk serve"

3. **Develop with full debugging support!**

**Note:** The debugger no longer tries to start containers - it assumes they're already running from setup.

### Reloading Data

If you need fresh production data:
```bash
./scripts/load-prod-data.sh
```

### Stopping

1. Stop backend/frontend (stop debugger in VSCode)
2. Stop dependencies:
   ```bash
   # Uses environment from RUST_ENV or defaults to development
   source scripts/load-env.sh
   docker compose -p hybrid_dev_env -f deploy/docker-compose.deps.yml --env-file config/.env.${RUST_ENV} down
   ```

## Benefits vs Full Docker

| Feature | Hybrid Dev | Full Docker |
|---------|------------|-------------|
| Code changes | ✅ Instant (no rebuild) | ⚠️ Requires rebuild |
| Debugging | ✅ Full debugger support | ⚠️ Limited |
| Hot reload | ✅ Frontend hot reload | ❌ No |
| Production data | ✅ Yes | ✅ Yes |
| Resource usage | ✅ Lower | ⚠️ Higher |
| Setup complexity | ⚠️ Slightly more | ✅ Simpler |

## Troubleshooting

### Dependencies won't start

```bash
# Check Docker is running
docker ps

# Check ports are free
lsof -i :50001  # ArangoDB
lsof -i :6379   # Redis

# Check environment file
cat config/.env.development
```

### Backend won't connect

```bash
# Verify services are accessible
curl http://localhost:50001/_api/version
redis-cli -h localhost -p 6379 ping

# Check environment variables in VSCode launch.json
# Make sure ARANGO_URL and REDIS_URL point to localhost
```

### Frontend won't build

```bash
# Install Trunk if missing
cargo install trunk

# Build CSS
cd frontend && npm run build:css:prod

# Check Trunk.toml configuration
```

## Next Steps

- See [Development Setup](DEVELOPMENT_SETUP.md) for more details
- See [Testing Setup](../testing/TESTING_SETUP.md) for testing with this setup
- See [Project Structure](PROJECT_STRUCTURE.md) for project organization

