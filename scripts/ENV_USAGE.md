# Environment Configuration Usage

All scripts now support both `config/.env.development` and `config/.env.production` through the `scripts/load-env.sh` helper.

## How It Works

The `load-env.sh` script automatically determines which environment file to use:

1. **Explicit argument**: `source scripts/load-env.sh production`
2. **RUST_ENV variable**: If `RUST_ENV` is set, it uses that environment
3. **Default**: Falls back to `development` if neither is specified

## Usage Examples

### Development (Default)
```bash
# Uses .env.development automatically
source scripts/load-env.sh
# or explicitly:
source scripts/load-env.sh development
```

### Production
```bash
# Explicitly use production
source scripts/load-env.sh production

# Or set RUST_ENV first
export RUST_ENV=production
source scripts/load-env.sh
```

## Scripts That Use load-env.sh

All these scripts automatically use the correct environment:

- `scripts/start-frontend.sh` - Uses development by default
- `scripts/setup-hybrid-dev.sh` - Uses development
- `scripts/start-hybrid-dev.sh` - Uses development
- `scripts/check-dev-setup.sh` - Uses development
- `scripts/load-prod-data.sh` - Uses development (for loading into dev)
- `scripts/backup-prod-db.sh` - Uses development (for local backups)

## Scripts That Explicitly Use Production

These scripts are production-specific and should use production environment:

- `scripts/build-prod-images.sh` - Explicitly uses `.env.production`
- `scripts/deploy-tested-images.sh` - Should use production
- `scripts/test-prod-containers.sh` - Should use production

## Environment Variables Loaded

The script loads all variables from the selected `.env` file and:

- Expands variable substitutions (e.g., `${ARANGODB_PORT}`)
- Sets defaults for ports if not specified
- Exports `RUST_ENV` for other scripts
- Handles URL construction (ARANGO_URL, BACKEND_URL, REDIS_URL)

## Port Configuration

Both development and production templates use the same default ports:
- `ARANGODB_PORT`: 50011
- `BACKEND_PORT`: 50012
- `FRONTEND_PORT`: 50013
- `REDIS_PORT`: 6379

You can override these in your `.env` files.

## Creating Environment Files

```bash
# Create development environment
./config/setup-env.sh development

# Create production environment
./config/setup-env.sh production
```

Then edit the created files in `config/.env.development` and `config/.env.production` with your specific values.

