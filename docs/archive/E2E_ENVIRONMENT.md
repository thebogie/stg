# E2E Test Environment Configuration

**All E2E test variables come from `config/.env.development`.**

## How It Works

### 1. Environment File Loading

The E2E test scripts use `scripts/load-env.sh` to load **all variables** from `config/.env.development`:

```bash
# In start-e2e-docker.sh
source "${SCRIPT_DIR}/load-env.sh" development
```

This loads:
- ✅ Database URLs: `ARANGO_URL`, `REDIS_URL`
- ✅ Database credentials: `ARANGO_USERNAME`, `ARANGO_PASSWORD`, `ARANGO_ROOT_PASSWORD`
- ✅ API keys: `BGG_API_TOKEN`, `GOOGLE_LOCATION_API`, etc.
- ✅ Ports: `ARANGODB_PORT`, `REDIS_PORT`, `BACKEND_PORT`
- ✅ All other configuration variables

### 2. Docker Compose

Docker Compose uses `--env-file` to load all variables:

```bash
docker compose \
  --env-file "$ENV_FILE" \  # Loads ALL variables from config/.env.development
  -f docker-compose.yaml \
  -f docker-compose.e2e.yml \
  up -d --build
```

The `docker-compose.yaml` and `docker-compose.e2e.yml` files reference these variables:
- `${ARANGO_URL}` - Database connection URL
- `${REDIS_URL}` - Redis connection URL
- `${BGG_API_TOKEN}` - BoardGameGeek API token
- `${GOOGLE_LOCATION_API}` - Google Location API key
- `${ARANGODB_PORT}`, `${REDIS_PORT}`, `${BACKEND_PORT}` - Ports
- All other variables from the env file

### 3. Port Override

**Only `FRONTEND_PORT` is overridden** to `8080` for Playwright compatibility:

```bash
export FRONTEND_PORT=8080  # Override for E2E tests
```

All other ports (`BACKEND_PORT`, `ARANGODB_PORT`, `REDIS_PORT`) come from `config/.env.development`.

## Configuration Flow

```
config/.env.development
    ↓
scripts/load-env.sh (loads all variables)
    ↓
start-e2e-docker.sh (exports variables, overrides FRONTEND_PORT=8080)
    ↓
docker compose --env-file (passes all variables to containers)
    ↓
Docker containers (use variables for configuration)
```

## Variables Used

### Database Configuration
- `ARANGO_URL` - ArangoDB connection URL
- `ARANGO_DB` - Database name
- `ARANGO_USERNAME` - Database username
- `ARANGO_PASSWORD` - Database password
- `ARANGO_ROOT_PASSWORD` - Root password
- `ARANGODB_PORT` - ArangoDB port

### Redis Configuration
- `REDIS_URL` - Redis connection URL
- `REDIS_PORT` - Redis port

### API Keys
- `BGG_API_TOKEN` - BoardGameGeek API token
- `GOOGLE_LOCATION_API` - Google Location API key
- Any other API keys in your env file

### Ports
- `FRONTEND_PORT` - Overridden to `8080` for E2E tests
- `BACKEND_PORT` - From env file (default: 50002)
- `ARANGODB_PORT` - From env file (default: 50011)
- `REDIS_PORT` - From env file (default: 6379)

### Other Variables
- All other variables in `config/.env.development` are available to containers

## Verifying Variables

To see what variables are loaded:

```bash
# Load environment
source scripts/load-env.sh development

# Check specific variables
echo "ARANGO_URL: $ARANGO_URL"
echo "REDIS_URL: $REDIS_URL"
echo "BACKEND_PORT: $BACKEND_PORT"
echo "BGG_API_TOKEN: ${BGG_API_TOKEN:0:10}..."  # Show first 10 chars

# List all relevant variables
env | grep -E "(ARANGO|REDIS|BACKEND|FRONTEND|API|GOOGLE|BGG)" | sort
```

## Important Notes

1. **Single Source of Truth**: `config/.env.development` is the single source for all configuration
2. **Only FRONTEND_PORT Overridden**: Only the frontend port is changed to 8080 for Playwright
3. **All Other Variables**: Come directly from the env file
4. **No Hardcoding**: No API keys, URLs, or credentials are hardcoded in scripts

## Troubleshooting

### Variables Not Loading

If variables aren't being used:

1. **Check env file exists**:
   ```bash
   ls -la config/.env.development
   ```

2. **Verify load-env.sh works**:
   ```bash
   source scripts/load-env.sh development
   echo $ARANGO_URL
   ```

3. **Check docker-compose uses env file**:
   ```bash
   # Should show --env-file in the command
   cat scripts/start-e2e-docker.sh | grep env-file
   ```

### Missing Variables

If a variable is missing:

1. **Add to `config/.env.development`**:
   ```bash
   # Edit the file
   nano config/.env.development
   
   # Add your variable
   MY_NEW_VAR=value
   ```

2. **Restart containers**:
   ```bash
   ./scripts/stop-e2e-docker.sh
   ./scripts/start-e2e-docker.sh
   ```

## Summary

✅ **All variables come from `config/.env.development`**  
✅ **Only `FRONTEND_PORT` is overridden to `8080`**  
✅ **No hardcoded values in scripts**  
✅ **Single source of truth for configuration**

