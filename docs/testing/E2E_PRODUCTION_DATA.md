# E2E Tests with Production Data

E2E tests can now use production backup data (`smacktalk.zip`) for more realistic testing scenarios.

## How It Works

When you run `just test-frontend-e2e` or `./scripts/start-e2e-docker.sh`, the script automatically:

1. **Starts Docker containers** (frontend, backend, ArangoDB, Redis)
2. **Waits for services** to be healthy
3. **Automatically loads production backup** if `smacktalk.zip` is found in `_build/backups/`

## Setup

### 1. Get Production Backup

Place the production backup file in one of these locations:

- `_build/backups/smacktalk.zip` (recommended)
- `../_backups/smacktalk.zip`
- Or specify path manually: `./scripts/load-e2e-data.sh /path/to/backup.zip`

### 2. Run E2E Tests

```bash
# Automatically loads production data if backup exists
just test-frontend-e2e

# Or manually start containers
./scripts/start-e2e-docker.sh
```

The script will automatically detect and load the backup after ArangoDB is ready.

## Manual Control

### Load Production Data Manually

```bash
# Load from default location (_build/backups/smacktalk.zip)
./scripts/load-e2e-data.sh

# Load from specific file
./scripts/load-e2e-data.sh /path/to/backup.zip
```

### Skip Loading Production Data

```bash
# Start E2E environment without loading production data
LOAD_PROD_DATA=0 ./scripts/start-e2e-docker.sh
```

## Benefits

Using production data for E2E tests provides:

1. **Realistic scenarios**: Tests run against real data structures and relationships
2. **Better coverage**: Tests exercise actual production data patterns
3. **User accounts**: Real user accounts exist (like `test@example.com` if it exists in production)
4. **Data relationships**: Venues, games, contests, and their relationships are preserved
5. **Edge cases**: Production data often contains edge cases that synthetic data misses

## Backup Format

The script supports:
- `.zip` files containing ArangoDB dumps
- `.tar.gz` or `.tar` files containing ArangoDB dumps
- Nested structures (e.g., `backup.zip` → `smacktalk/` → database files)

## Troubleshooting

### Backup Not Found

If the backup isn't found, E2E tests will continue with an empty database:

```
⚠️  Backup file not found: not specified
💡 Usage: ./scripts/load-e2e-data.sh [backup-file]
💡 Or place smacktalk.zip in _build/backups/
   Continuing without loading data...
```

This is fine - tests will create their own data as needed.

### Restore Fails

If restore fails, check:
1. ArangoDB container is running: `docker ps | grep arangodb`
2. Backup file is valid: `unzip -t _build/backups/smacktalk.zip`
3. Database credentials in `config/.env.development` are correct

### Data Conflicts

If you need a fresh start:
```bash
# Stop and remove containers (this wipes the database)
docker compose -p e2e_env -f deploy/docker-compose.yaml -f deploy/docker-compose.prod.yml -f deploy/docker-compose.stg_prod.yml -f deploy/docker-compose.e2e.yml down -v

# Restart and reload
./scripts/start-e2e-docker.sh
```

## Integration with CI/CD

For CI/CD, you can:

1. **Download backup** from artifact storage
2. **Place in `_build/backups/smacktalk.zip`**
3. **Run E2E tests** - they'll automatically load the backup

Or skip loading in CI:
```bash
LOAD_PROD_DATA=0 just test-frontend-e2e
```
