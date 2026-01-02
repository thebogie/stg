# Documentation Organization

This document explains how documentation is organized in this project.

## Structure

```
docs/
├── README.md                    # Documentation index
├── setup/                       # Setup and development guides
│   ├── DEVELOPMENT_SETUP.md
│   ├── PROJECT_STRUCTURE.md
│   └── MIGRATION_GUIDE.md
├── testing/                     # Testing documentation
│   ├── TEST_REPORTING_GUIDE.md
│   ├── TEST_RESULTS_SUMMARY.md
│   ├── TESTING_TIERS.md
│   ├── TESTING_STATUS.md
│   ├── TESTING_ARCHITECTURE.md
│   ├── TESTING_SETUP.md
│   ├── TESTING.md
│   ├── ADVANCED_TESTING.md
│   ├── TESTCONTAINERS_COMPLETE.md
│   ├── PRODUCTION_READINESS_ASSESSMENT.md
│   └── PRODUCTION_READINESS_ACTION_PLAN.md
├── api/                         # API documentation
│   └── AUTHENTICATION_API.md
├── architecture/                # Architecture documentation
│   ├── CLIENT_ANALYTICS_ARCHITECTURE.md
│   └── CLIENT_ANALYTICS_README.md
└── [other docs]                 # Feature-specific documentation
```

## Categories

### Setup (`docs/setup/`)
Development environment setup, project structure, and migration guides.

### Testing (`docs/testing/`)
All testing-related documentation including setup, architecture, reporting, and production readiness.

### API (`docs/api/`)
API endpoint documentation and usage guides.

### Architecture (`docs/architecture/`)
System design documents and architecture decisions.

### Root `docs/`
Feature-specific documentation that doesn't fit into categories above.

## Root-Level Documentation

Some documentation remains at the project root for easy discovery:

- **README.md** - Main project overview
- **README_QUICK_START.md** - Quick start guide

## Deployment Documentation

Deployment-specific documentation lives in the `deploy/` directory:

- `deploy/PRODUCTION_DEPLOYMENT.md`
- `deploy/INDUSTRY_STANDARDS.md`

This keeps deployment docs close to deployment configuration files.

## Finding Documentation

1. **Start here**: `docs/README.md` - Complete documentation index
2. **Quick start**: `README_QUICK_START.md` (root)
3. **Development**: `docs/setup/DEVELOPMENT_SETUP.md`
4. **Testing**: `docs/testing/TESTING_SETUP.md`
5. **Deployment**: `deploy/PRODUCTION_DEPLOYMENT.md`

## Adding New Documentation

When adding new documentation:

1. **Choose the right category**:
   - Setup guides → `docs/setup/`
   - Testing docs → `docs/testing/`
   - API docs → `docs/api/`
   - Architecture → `docs/architecture/`
   - Feature-specific → `docs/` root

2. **Update the index**: Add a link in `docs/README.md`

3. **Follow naming**: Use `UPPER_SNAKE_CASE.md` for major documents

4. **Update cross-references**: If you move files, update links in related docs

