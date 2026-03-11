# Quick Start Guide

## For Development

### Option 1: Full Docker (Production-like)
```bash
./deploy/deploy.sh --env development --build
```

### Option 2: Hybrid Dev (Recommended - with Debugger)
```bash
# 1. Setup environment (one command)
./scripts/setup-hybrid-dev.sh
# This starts containers AND loads data (if available)

# 2. Start backend in VSCode:
#    Debug panel → "Debug Backend (Hybrid Dev)" → F5

# 3. Start frontend:
#    ./scripts/start-frontend.sh
#    Or: Ctrl+Shift+P → "Tasks: Run Task" → "frontend: trunk serve"
```

**See [Hybrid Development Guide](docs/setup/HYBRID_DEVELOPMENT.md) for details.**

## For Production

1. **Setup environment:**
   ```bash
   ./config/setup-env.sh production
   # Edit config/.env.production with production values
   ```

2. **Deploy:**
   ```bash
   ./deploy/deploy.sh --env production --build
   ```

3. **Monitor:**
   ```bash
   ./deploy/deploy.sh --env production --status
   ./deploy/deploy.sh --env production --logs
   ```

## Common Commands

```bash
# Start development (full Docker)
./deploy/deploy.sh --env development --build

# Setup hybrid dev (dependencies + data)
./scripts/setup-hybrid-dev.sh

# Stop services
./deploy/deploy.sh --env development --down

# View logs
./deploy/deploy.sh --env development --logs

# Check status
./deploy/deploy.sh --env development --status
```

## More Information

- **Hybrid Development**: See `docs/setup/HYBRID_DEVELOPMENT.md` (recommended for active dev)
- **Project Structure**: See `docs/setup/PROJECT_STRUCTURE.md`
- **Development Setup**: See `docs/setup/DEVELOPMENT_SETUP.md`
- **Migration Guide**: See `docs/setup/MIGRATION_GUIDE.md` (if migrating from old structure)
- **Full Documentation**: See `docs/README.md`
