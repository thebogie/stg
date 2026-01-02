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

# 3. Export tested images (1-2 min)
./scripts/export-tested-images.sh

# 4. Transfer to production (2-5 min)
scp _build/artifacts/*.tar.gz* user@production-server:/tmp/

# 5. Deploy on production server (2-3 min)
ssh production-server
cd /path/to/stg_rd
./scripts/deploy-tested-images.sh --version v<version> --image-dir /tmp
```

**Total time**: ~15-50 minutes (mostly automated)

See [DAILY_WORKFLOW.md](docs/DAILY_WORKFLOW.md) for detailed step-by-step guide with explanations.

## 📋 Complete Workflow

For detailed instructions, see: [TEST_THEN_DEPLOY_WORKFLOW.md](docs/TEST_THEN_DEPLOY_WORKFLOW.md)

## 🎯 Master Workflow Script

Or use the master script that orchestrates everything:

```bash
# Build → Test → Export
./scripts/workflow-test-then-deploy.sh

# Or skip tests (not recommended)
./scripts/workflow-test-then-deploy.sh --skip-tests
```

## 📚 Scripts Reference

| Script | Purpose |
|--------|---------|
| `build-prod-images.sh` | Build production Docker images locally |
| `test-prod-containers.sh` | Test production containers with production data |
| `test-migrations-workflow.sh` | Test migrations from scratch (wipe → migrate) |
| `test-migrations-on-existing-data.sh` | Test migrations on existing data (restore → migrate) |
| `export-tested-images.sh` | Export tested images for deployment |
| `backup-prod-db.sh` | Backup production database |
| `deploy-tested-images.sh` | Deploy tested images to production |
| `deploy-with-migrations.sh` | Deploy with migrations (production workflow) |
| `workflow-test-then-deploy.sh` | Master workflow script |

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

