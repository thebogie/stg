# Development Setup Guide

This guide explains how to set up VSCode and Docker containers for local development that matches your production environment.

## Overview

The development setup uses:
- **docker-compose.yaml** - Base production-like configuration
- **docker-compose.dev.yml** - Development overrides (volume mounts, dev environment variables)
- **VSCode configurations** - Debugging, tasks, and workspace settings

## Prerequisites

1. **Docker & Docker Compose** installed and running
2. **VSCode** with recommended extensions (see `.vscode/extensions.json`)
3. **Rust toolchain** (for local development option)

## Quick Start

### 1. Install VSCode Extensions

Open VSCode in this workspace and install recommended extensions:
- Press `Ctrl+Shift+P` (or `Cmd+Shift+P` on Mac)
- Type "Extensions: Show Recommended Extensions"
- Click "Install All"

Or manually install:
- Rust Analyzer
- CodeLLDB (for debugging)
- Docker extension
- TOML support
- YAML support

### 2. Create Environment File

```bash
# Use the setup script
./config/setup-env.sh development

# Then edit config/.env.development with your values
```

**Required environment variables** (see `config/env.development.template` for full list):
- `ENV_FILE` - Path to your .env file (usually `.env.development`)
- `VOLUME_PATH` - Where to store Docker volumes (e.g., `./docker-data`)
- `FRONTEND_PORT` - Frontend port (default: `50003`)
- `BACKEND_PORT` - Backend port (default: `50002`)
- `ARANGODB_PORT` - ArangoDB port (default: `50001`)
- `REDIS_PORT` - Redis port (default: `6379`)
- `ARANGO_ROOT_PASSWORD` - ArangoDB root password (change from default!)
- `ARANGO_PASSWORD` - ArangoDB user password (change from default!)

### 3. Start Docker Containers

**Using VSCode Tasks** (Recommended):
1. Press `Ctrl+Shift+P` (or `Cmd+Shift+P`)
2. Type "Tasks: Run Task"
3. Select "docker-compose: up (build)"

**Using Terminal**:
```bash
./deploy/deploy.sh --env development --build
```

### 4. Verify Services

Check that all services are running:
```bash
./deploy/deploy.sh --env development --status
```

View logs:
```bash
# All services
./deploy/deploy.sh --env development --logs

# Specific service
./deploy/deploy.sh --env development --logs backend
```

Or use VSCode tasks: "docker-compose: logs" or "docker-compose: logs (backend)"

## Development Workflows

### Option 1: Full Docker (Production-like Testing)

Run everything in Docker containers, matching production:

```bash
./deploy/deploy.sh --env development --build
```

**Pros:**
- Matches production environment exactly
- Isolated dependencies
- Easy to reset/clean state

**Cons:**
- Slower iteration (requires rebuild for code changes)
- More resource intensive

**Best for:**
- Integration testing
- Testing production-like deployments
- CI/CD validation

### Option 2: Hybrid (Recommended for Active Development)

Run dependencies (Redis, ArangoDB) in Docker, code locally:

```bash
# Start only dependencies (using docker compose directly)
cd deploy
export ENV_FILE=../config/.env.development
docker compose -f docker-compose.yaml up -d redis arangodb
cd ..

# In terminal 1: Run backend locally
export ENV_FILE=.env.development
source .env.development  # Load environment variables
cargo run --package backend

# In terminal 2: Run frontend locally
cd frontend
trunk serve --no-default-features --features frontend
```

**Pros:**
- Fast iteration (no rebuild needed)
- Hot reload for frontend
- Easy debugging

**Cons:**
- Requires local Rust/Node setup
- Slightly different from production

**Best for:**
- Active feature development
- Debugging
- Rapid iteration

### Option 3: Local Everything

Run everything locally (requires local Redis and ArangoDB installations):

```bash
# Install Redis and ArangoDB locally, then:
cargo run --package backend
cd frontend && trunk serve
```

## VSCode Features

### Tasks

Access via `Ctrl+Shift+P` → "Tasks: Run Task":

- **docker-compose: up** - Start containers
- **docker-compose: up (build)** - Build and start containers
- **docker-compose: down** - Stop containers
- **docker-compose: logs** - View all logs
- **docker-compose: logs (backend)** - View backend logs
- **docker-compose: logs (frontend)** - View frontend logs
- **docker-compose: restart** - Restart all containers
- **cargo: build (backend)** - Build backend locally
- **cargo: test (workspace)** - Run all Rust tests

### Debugging

1. **Debug Backend Locally:**
   - Start dependencies: `docker compose up -d redis arangodb`
   - Set breakpoints in your code
   - Select "Run Backend Locally (Dev)" from debug panel
   - Press F5 to start debugging

2. **Debug Backend in Container:**
   - Start all containers
   - Set breakpoints
   - Select "Attach to Backend Container" from debug panel
   - Note: Requires LLDB debugger support in container

3. **Run Tests:**
   - Select "Run Backend Tests" from debug panel
   - Or use task "cargo: test (workspace)"

### Settings

Workspace settings (`.vscode/settings.json`) include:
- Rust analyzer configuration
- Code formatting on save
- Docker compose file references
- File associations

## File Structure

```
stg_rd/
├── .vscode/                    # VSCode configuration
│   ├── launch.json            # Debug configurations
│   ├── tasks.json             # Task definitions
│   ├── settings.json          # Workspace settings
│   ├── extensions.json        # Recommended extensions
│   └── README.md              # VSCode setup details
├── config/                    # Environment configuration
│   ├── env.development.template
│   ├── env.production.template
│   └── setup-env.sh
├── deploy/                    # Deployment configuration
│   ├── docker-compose.yaml        # Base config
│   ├── docker-compose.dev.yml     # Dev overrides
│   ├── docker-compose.prod.yml    # Prod overrides
│   └── deploy.sh                  # Unified deployment script
└── DEVELOPMENT_SETUP.md       # This file
```

## Troubleshooting

### Containers won't start

1. Check environment variables:
   ```bash
   cat config/.env.development
   ```

2. Check Docker is running:
   ```bash
   docker ps
   ```

3. Check port conflicts:
   ```bash
   # Check if ports are in use
   lsof -i :50002  # Backend
   lsof -i :50003  # Frontend
   lsof -i :50001  # ArangoDB
   lsof -i :6379   # Redis
   ```

### Backend can't connect to database

1. Verify ArangoDB is running:
   ```bash
   docker compose -f docker-compose.yaml ps arangodb
   ```

2. Check connection URL in `config/.env.development`:
   - For containers: `http://arangodb:8529`
   - For local: `http://localhost:50001`

3. Check credentials match in `.env.development`

### Frontend can't connect to backend

1. Verify backend is running:
   ```bash
   curl http://localhost:50002/health
   ```

2. Check CORS settings in backend middleware
3. Verify `BACKEND_URL` in frontend environment

### Volume mount issues

If you see permission errors with volume mounts:
```bash
# Fix permissions (Linux/Mac)
sudo chown -R $USER:$USER ./docker-data
```

### Clean slate

To start fresh:
```bash
# Stop and remove containers
./deploy/deploy.sh --env development --down

# Remove volumes (WARNING: deletes data)
rm -rf ./docker-data

# Rebuild
./deploy/deploy.sh --env development --build
```

## Production Parity

The `deploy/docker-compose.dev.yml` file includes volume mounts for faster development. For **true production parity**:

1. Use production environment: `./deploy/deploy.sh --env production --build`
2. Production compose file (`deploy/docker-compose.prod.yml`) has no volume mounts
3. Code is baked into images, matching production exactly

This ensures you're testing the exact same setup as production.

## Next Steps

- See `.vscode/README.md` for detailed VSCode configuration
- See `../README.md` for project overview
- See `../testing/TESTING_SETUP.md` for testing configuration

