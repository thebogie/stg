# E2E Tests with Docker

E2E tests now use Docker containers instead of running Trunk locally. This provides a more realistic testing environment that matches production.

## How It Works

When you run `just test-frontend-e2e`, Playwright:

1. **Starts Docker containers** via `scripts/start-e2e-docker.sh`:
   - Frontend (Nginx serving built WASM) on port 8080
   - Backend (Rust API server) on port 50002
   - ArangoDB (database)
   - Redis (session store)

2. **Waits for services** to be healthy and ready

3. **Runs E2E tests** against the running containers

4. **Reuses containers** on subsequent runs (unless `CI=true`)

## Prerequisites

### 1. Environment File

Ensure you have `config/.env.development` configured:

```bash
# If missing, create from template
./config/setup-env.sh development
```

### 2. Docker

Ensure Docker is installed and running:

```bash
docker --version
docker ps  # Should work without errors
```

## Running E2E Tests

### Basic Usage

```bash
# Run E2E tests (starts Docker automatically)
just test-frontend-e2e

# Or directly
npx playwright test
```

### Manual Control

```bash
# Start containers manually
./scripts/start-e2e-docker.sh

# Run tests (will reuse existing containers)
npx playwright test

# Stop containers when done
./scripts/stop-e2e-docker.sh
# or
just test-frontend-e2e-stop
```

## What Gets Started

The E2E Docker setup starts:

- **Frontend**: Production-like Nginx server on port 8080
- **Backend**: Full API server on port 50002
- **ArangoDB**: Database on port 50011 (default)
- **Redis**: Session store on port 6379 (default)

All services are connected via Docker network and ready for testing.

## Configuration

### Ports

E2E tests use fixed ports:
- Frontend: `8080` (mapped from container port 8080)
- Backend: `50002` (mapped from container port 50002)

These are configured in `deploy/docker-compose.e2e.yml`.

### Environment Variables

**All variables come from `config/.env.development`** via `scripts/load-env.sh`.

The E2E setup loads ALL variables from the environment file, including:
- Database URLs: `ARANGO_URL`, `REDIS_URL`
- Database credentials: `ARANGO_USERNAME`, `ARANGO_PASSWORD`, `ARANGO_ROOT_PASSWORD`
- API keys: `BGG_API_TOKEN`, `GOOGLE_LOCATION_API`, etc.
- Ports: `ARANGODB_PORT`, `REDIS_PORT`, `BACKEND_PORT`
- All other configuration variables

**Only `FRONTEND_PORT` is overridden** to `8080` for Playwright compatibility.

To see what variables are loaded:
```bash
source scripts/load-env.sh development
env | grep -E "(ARANGO|REDIS|BACKEND|FRONTEND|API)"
```

## Troubleshooting

### Containers Don't Start

**Problem**: `start-e2e-docker.sh` fails

**Solutions**:
- Check Docker is running: `docker ps`
- Check environment file exists: `ls config/.env.development`
- Check ports are available: `lsof -i :8080` and `lsof -i :50002`
- View logs: `docker compose -f deploy/docker-compose.yaml -f deploy/docker-compose.e2e.yml logs`

### Frontend Not Accessible

**Problem**: Tests can't connect to `http://localhost:8080`

**Solutions**:
- Check container is running: `docker ps | grep frontend`
- Check container logs: `docker logs frontend`
- Verify port mapping: `docker port frontend`
- Wait longer: First build takes 3-5 minutes

### Backend Not Ready

**Problem**: Backend health check fails

**Solutions**:
- Check backend logs: `docker logs backend`
- Check database connection: `docker logs arangodb`
- Verify environment variables in `.env.development`
- Increase timeout in `start-e2e-docker.sh` if needed

### Port Conflicts

**Problem**: Port already in use

**Solutions**:
- Stop existing containers: `./scripts/stop-e2e-docker.sh`
- Check what's using the port: `lsof -i :8080`
- Use different ports by modifying `docker-compose.e2e.yml`

## Cleanup

### Stop Containers

```bash
# Stop E2E containers
./scripts/stop-e2e-docker.sh

# Or manually
cd deploy
docker compose -f docker-compose.yaml -f docker-compose.e2e.yml down
```

### Clean Everything

```bash
# Stop and remove containers, volumes, and images
cd deploy
docker compose -f docker-compose.yaml -f docker-compose.e2e.yml down -v --rmi all
```

## Advantages Over Trunk

Using Docker for E2E tests provides:

✅ **Production-like environment** - Tests run against the same setup as production
✅ **Full stack testing** - Backend, frontend, and databases all running
✅ **Consistent builds** - Same Docker images as production
✅ **Isolated environment** - No conflicts with local development
✅ **Faster subsequent runs** - Containers are reused

## CI/CD Integration

For CI, set `CI=true` to ensure fresh containers:

```bash
CI=true npx playwright test
```

This prevents reusing existing containers and ensures clean test runs.

