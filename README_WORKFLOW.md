# Test-Then-Deploy Workflow - Quick Start

This project uses a **test-then-deploy** workflow to ensure production runs exactly what you've tested.

## 🚀 Quick Start (80% Case - Regular Development)

### Typical Daily Workflow

```bash
# 1. Build production images (5-10 min)
./scripts/build-prod-images.sh

# 2. Test locally with test database (5-30 min)
./scripts/test-prod-containers.sh --use-test-db
# Test your changes in browser/API

# 3. Push to Docker Hub (automatically done by build-test-push.sh)
# Images are now on Docker Hub

# 4. Deploy on production server (2-3 min)
ssh production-server
cd ~/stg/repo
git pull  # Get latest deploy script
./scripts/deploy-production.sh --version v<version>
```

**Total time**: ~15-50 minutes (mostly automated)

See [DAILY_WORKFLOW.md](docs/DAILY_WORKFLOW.md) for detailed step-by-step guide with explanations.

## 📋 Complete Workflow

For detailed instructions, see: [TEST_THEN_DEPLOY_WORKFLOW.md](docs/TEST_THEN_DEPLOY_WORKFLOW.md)

## 🎯 Master Workflow Script

Use the simplified workflow:

```bash
# Build → Test → Push to Docker Hub
./scripts/build-test-push.sh
```

## 📚 Scripts Reference

| Script | Purpose |
|--------|---------|
| `build-test-push.sh` | **Main dev workflow**: Build, test, push to Docker Hub |
| `deploy-production.sh` | **Main prod workflow**: Pull from Docker Hub, deploy |
| `build-prod-images.sh` | Build production Docker images (used by build-test-push.sh) |
| `test-migrations-workflow.sh` | Test migrations from scratch (wipe → migrate) |
| `test-migrations-on-existing-data.sh` | Test migrations on existing data (restore → migrate) |
| `backup-prod-db.sh` | Backup production database |
| `test-dev.sh` | Quick unit tests during development |
| `test-integration.sh` | Integration tests with testcontainers |

## 🔄 Migration Testing

Migrations need to work in two scenarios:
1. **Fresh install**: `./scripts/test-migrations-workflow.sh` (wipe → migrate)
2. **Existing data**: `./scripts/test-migrations-on-existing-data.sh` (restore → migrate)

See [MIGRATION_TESTING_WORKFLOW.md](docs/MIGRATION_TESTING_WORKFLOW.md) for details.

## 🔑 Key Benefits

✅ **Zero Delta**: Exact images tested = exact images deployed  
✅ **No Git Pull in Production**: Only tested artifacts deployed  
✅ **Faster Deployments**: No build time on production  
✅ **Easy Rollback**: Tag previous version and redeploy  
✅ **Data Safety**: Volumes persist, data never lost  

## 📖 Documentation

- [Complete CI/CD Workflow](docs/CI_CD_WORKFLOW.md) - **Start here for full workflow**
- [Test-Then-Deploy Guide](docs/TEST_THEN_DEPLOY_WORKFLOW.md) - Detailed deployment workflow
- [Daily Workflow](docs/DAILY_WORKFLOW.md) - Regular development workflow
- [Hybrid Development](HYBRID_DEV_QUICK_START.md) - Fast development with debugger
- [Production Deployment](deploy/PRODUCTION_DEPLOYMENT.md) - Production setup guide
- [Migrations Guide](migrations/README.md) - Database migrations

