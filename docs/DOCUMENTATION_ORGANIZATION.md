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
│   ├── HOW_TO_RUN_TESTS.md
│   ├── E2E_TESTING_GUIDE.md
│   ├── TESTING_ARCHITECTURE.md
│   ├── TESTING_SETUP.md
│   └── [other testing docs]
├── api/                         # API documentation
│   └── AUTHENTICATION_API.md
└── architecture/                # Architecture documentation
    ├── CLIENT_ANALYTICS_ARCHITECTURE.md
    └── CLIENT_ANALYTICS_README.md
```

## Root-Level Documentation

Some documentation remains at the project root for easy discovery:

- **README.md** - Main project overview
- **README_QUICK_START.md** - Quick start guide
- **DEPLOY_TO_PRODUCTION.md** - Production deployment guide (Docker Hub method)

## Deployment Documentation

**Primary deployment guide**: `DEPLOY_TO_PRODUCTION.md` (project root)

This is the authoritative guide for deploying to production using Docker Hub with a single private repository. All deployment steps are documented here to avoid duplication.

**Workflow documents** (reference the primary guide):
- `docs/DAILY_WORKFLOW.md` - Daily development workflow → references DEPLOY_TO_PRODUCTION.md for deployment
- `docs/TEST_THEN_DEPLOY_WORKFLOW.md` - Test-then-deploy workflow → references DEPLOY_TO_PRODUCTION.md for deployment
- `docs/CI_CD_WORKFLOW.md` - CI/CD pipeline → references DEPLOY_TO_PRODUCTION.md for deployment

This structure eliminates duplication - update deployment steps in one place (DEPLOY_TO_PRODUCTION.md), and all workflow docs stay current.

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

## Finding Documentation

1. **Start here**: `docs/README.md` - Complete documentation index
2. **Quick start**: `README_QUICK_START.md` (root)
3. **Development**: `docs/setup/DEVELOPMENT_SETUP.md`
4. **Testing**: `docs/testing/HOW_TO_RUN_TESTS.md`
5. **Deployment**: `DEPLOY_TO_PRODUCTION.md` (root) - **Primary deployment guide**

## Adding New Documentation

When adding new documentation:

1. **Choose the right category**:
   - Setup guides → `docs/setup/`
   - Testing docs → `docs/testing/`
   - API docs → `docs/api/`
   - Architecture → `docs/architecture/`
   - Feature-specific → `docs/` root
   - Deployment → Update `DEPLOY_TO_PRODUCTION.md` (root)

2. **Update the index**: Add a link in `docs/README.md`

3. **Follow naming**: Use `UPPER_SNAKE_CASE.md` for major documents

4. **Update cross-references**: If you move files, update links in related docs

5. **Avoid duplication**: Check if similar content exists before creating new docs
