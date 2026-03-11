# Hybrid Development - Quick Debug Setup

**Get your development environment running with full debugger support in 3 steps.**

## One-Command Setup

```bash
./scripts/setup-hybrid-dev.sh
```

This single command:
1. ✅ Starts ArangoDB and Redis in Docker
2. ✅ Loads production data (if available)
3. ✅ Shows you next steps

**Skip data load:**
```bash
./scripts/setup-hybrid-dev.sh --skip-data
```

---

## Start Backend (with Debugger)

### Option 1: VSCode (Recommended)
1. Press **F5** (or click Debug icon)
2. Select **"Debug Backend (Hybrid Dev)"** from dropdown
3. Press **F5** to start
4. Set breakpoints anywhere in your Rust code
5. Backend connects to Docker services automatically

### Option 2: Terminal
```bash
# Load environment variables
source scripts/load-env.sh

# Run backend
export RUST_LOG=debug
cargo run --package backend --bin backend
```

---

## Start Frontend

```bash
./scripts/start-frontend.sh
```

Frontend will be available at: **http://localhost:${FRONTEND_PORT}**  
(Check your port: `source scripts/load-env.sh && echo $FRONTEND_PORT`)

---

## That's It!

You now have:
- ✅ Backend running with full debugger support
- ✅ Frontend with hot reload
- ✅ Production data in ArangoDB
- ✅ Breakpoints, step-through, variable inspection

---

## Daily Workflow

**First time:**
```bash
./scripts/setup-hybrid-dev.sh
```

**Every day:**
1. Start backend: **F5** in VSCode (containers already running)
2. Start frontend: `./scripts/start-frontend.sh`
3. Debug!

**Stop:**
- Backend: Stop debugger (Shift+F5)
- Frontend: Ctrl+C in terminal
- Dependencies: Leave running (they auto-restart)

---

## Troubleshooting

**Backend won't start:**
```bash
# Check services are running
docker ps | grep -E "arangodb-dev|redis-dev"

# Check ports
source scripts/load-env.sh
echo "ArangoDB: $ARANGODB_PORT, Redis: $REDIS_PORT"
```

**Port conflicts:**
```bash
source scripts/load-env.sh
lsof -i :${BACKEND_PORT}  # Check what's using backend port
```

**Check everything:**
```bash
./scripts/check-dev-setup.sh
```

---

## Full Documentation

- [Hybrid Development Guide](docs/setup/HYBRID_DEVELOPMENT.md) - Complete details
- [Environment Usage](scripts/ENV_USAGE.md) - Environment configuration
- [CI/CD Workflow](docs/CI_CD_WORKFLOW.md) - Production release process

