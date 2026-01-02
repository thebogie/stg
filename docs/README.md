# Documentation Index

This directory contains all project documentation organized by category.

## 📚 Documentation Structure

```
docs/
├── README.md                    # This file - documentation index
├── setup/                        # Setup and development guides
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
├── ADMIN_AUTHORIZATION_SYSTEM.md
├── BACKEND_SCHEDULER_IMPLEMENTATION.md
├── DOCUMENTATION_CLEANUP.md
├── DOCUMENTATION_ORGANIZATION.md
├── GLICKO2_RATINGS_IMPLEMENTATION.md
├── NEXTEST_QUICK_REFERENCE.md
├── TEST_REPORTS.md
└── version-system.md
```

## Quick Links

### Getting Started
- **[Quick Start Guide](../README_QUICK_START.md)** - Get up and running quickly
- **[Development Setup Guide](setup/DEVELOPMENT_SETUP.md)** - Detailed development setup
- **[Project Structure](setup/PROJECT_STRUCTURE.md)** - Project organization
- **[Migration Guide](setup/MIGRATION_GUIDE.md)** - Migrating from old structure
- **[Documentation Organization](DOCUMENTATION_ORGANIZATION.md)** - How docs are organized

### Testing
- **[Testing Setup](testing/TESTING_SETUP.md)** - How to run tests
- **[Testing Architecture](testing/TESTING_ARCHITECTURE.md)** - Testing system design
- **[Testing Tiers](testing/TESTING_TIERS.md)** - Test coverage levels
- **[Advanced Testing](testing/ADVANCED_TESTING.md)** - Advanced testing scenarios and factories
- **[Test Reporting Guide](testing/TEST_REPORTING_GUIDE.md)** - How to report test results
- **[Production Readiness](testing/PRODUCTION_READINESS_ASSESSMENT.md)** - Production readiness checklist

### API Documentation
- **[Authentication API](api/AUTHENTICATION_API.md)** - Authentication endpoints

### Architecture
- **[Client Analytics Architecture](architecture/CLIENT_ANALYTICS_ARCHITECTURE.md)** - Analytics system design
- **[Glicko2 Ratings](GLICKO2_RATINGS_IMPLEMENTATION.md)** - Rating system implementation
- **[Backend Scheduler](BACKEND_SCHEDULER_IMPLEMENTATION.md)** - Scheduler implementation
- **[Admin Authorization](ADMIN_AUTHORIZATION_SYSTEM.md)** - Admin system design

### Deployment
- **[Production Deployment](../deploy/PRODUCTION_DEPLOYMENT.md)** - Production deployment guide
- **[Industry Standards](../deploy/INDUSTRY_STANDARDS.md)** - Deployment best practices

## Documentation Categories

### Setup (`setup/`)
Guides for setting up the development environment, understanding project structure, and migrating from old setups.

### Testing (`testing/`)
Comprehensive testing documentation including setup, architecture, reporting, and production readiness.

### API (`api/`)
API endpoint documentation and usage guides.

### Architecture (`architecture/`)
System design documents, architecture decisions, and implementation details.

## Contributing to Documentation

When adding new documentation:

1. **Place it in the appropriate category directory**
2. **Update this README.md** with a link
3. **Follow existing naming conventions** (UPPER_SNAKE_CASE.md for major docs)
4. **Include a brief description** in the index

## Finding Documentation

- **Quick Start**: See [README_QUICK_START.md](../README_QUICK_START.md) in project root
- **Development**: Start with [Development Setup](setup/DEVELOPMENT_SETUP.md)
- **Testing**: Start with [Testing Setup](testing/TESTING_SETUP.md)
- **Deployment**: See [deploy/](../deploy/) directory

