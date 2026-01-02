# Project Structure

This document describes the organized structure of the stg_rd project.

## Directory Layout

```
stg_rd/
├── backend/              # Rust backend API
├── frontend/             # WebAssembly frontend (Yew)
├── shared/               # Shared models and DTOs
├── scripts/              # Utility scripts (migrations, data loading, etc.)
├── testing/              # Integration testing crate
├── migrations/           # Database migrations
├── dataload/             # Data loading utilities
├── docs/                 # Project documentation
│
├── config/               # Environment configuration files
│   ├── env.development.template
│   ├── env.production.template
│   ├── .env.development  # (gitignored - create from template)
│   ├── .env.production  # (gitignored - create from template)
│   └── setup-env.sh     # Script to create .env files from templates
│
├── deploy/               # Deployment configuration
│   ├── docker-compose.yaml      # Base docker-compose configuration
│   ├── docker-compose.dev.yml   # Development overrides
│   ├── docker-compose.prod.yml  # Production overrides
│   └── deploy.sh                # Unified deployment script
│
├── .vscode/              # VSCode workspace configuration
│   ├── launch.json      # Debug configurations
│   ├── tasks.json        # Task definitions
│   ├── settings.json    # Workspace settings
│   └── extensions.json  # Recommended extensions
│
├── Cargo.toml            # Rust workspace configuration
├── README.md             # Project overview
├── DEVELOPMENT_SETUP.md  # Development setup guide
├── PROJECT_STRUCTURE.md  # This file
└── MIGRATION_GUIDE.md    # Migration from old structure
```

## Key Directories

### `config/` - Environment Configuration

All environment files are stored here:
- **Templates** (`env.*.template`): Safe to commit, contain example values
- **Actual files** (`.env.*`): Gitignored, contain real secrets

**Setup:**
```bash
# Create development environment
./config/setup-env.sh development

# Create production environment
./config/setup-env.sh production
```

### `deploy/` - Deployment Configuration

All Docker Compose files and deployment scripts:
- **docker-compose.yaml**: Base configuration (shared by dev and prod)
- **docker-compose.dev.yml**: Development overrides (volume mounts, dev settings)
- **docker-compose.prod.yml**: Production overrides (no volumes, prod settings)
- **deploy.sh**: Unified deployment script

**Usage:**
```bash
# Development
./deploy/deploy.sh --env development --build

# Production
./deploy/deploy.sh --env production --build
```

### Cargo Workspace

The project uses a Rust workspace with the following members:
- `backend` - Main API server
- `frontend` - WebAssembly frontend
- `shared` - Shared types and models
- `testing` - Integration test utilities
- `migrations` - Database migration tools
- `scripts` - Utility scripts (Rust-based)
- `dataload` - Data loading utilities

## Environment Files

### Development (`config/.env.development`)

- Created from `config/env.development.template`
- Used for local development
- Contains development-specific settings
- Volume mounts enabled for hot reload

### Production (`config/.env.production`)

- Created from `config/env.production.template`
- Used for production deployments
- Contains production-specific settings
- No volume mounts (code baked into images)
- **Never commit this file!**

## Deployment Workflow

### Development

1. **Setup environment:**
   ```bash
   ./config/setup-env.sh development
   # Edit config/.env.development with your values
   ```

2. **Start services:**
   ```bash
   ./deploy/deploy.sh --env development --build
   ```

3. **View logs:**
   ```bash
   ./deploy/deploy.sh --env development --logs
   ```

### Production

1. **Setup environment:**
   ```bash
   ./config/setup-env.sh production
   # Edit config/.env.production with production values
   # - Use strong passwords
   - Use absolute paths for VOLUME_PATH
   - Configure production ports
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

## Migration from Old Structure

If you're migrating from the old structure:

1. **Environment files:**
   - Old: `.env.development` at root or `../stg_dev/.env.development`
   - New: `config/.env.development`
   - Action: Run `./config/setup-env.sh development` and copy your values

2. **Docker Compose:**
   - Old: `docker-compose.yaml` and `docker-compose.dev.yml` at root
   - New: `deploy/docker-compose.yaml` and `deploy/docker-compose.dev.yml`
   - Action: Use `./deploy/deploy.sh` instead of direct docker-compose commands

3. **Production scripts:**
   - Old: `_prod/stg.sh` with hardcoded paths
   - New: `./deploy/deploy.sh --env production`
   - Action: Update your production deployment process

## Benefits of This Structure

1. **Clear separation** between dev and prod configurations
2. **Easy deployment** - single script for all environments
3. **Safe templates** - commit templates, ignore secrets
4. **Organized** - related files grouped together
5. **Scalable** - easy to add new environments (staging, etc.)

