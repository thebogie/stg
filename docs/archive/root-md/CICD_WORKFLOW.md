# Industry-Standard CI/CD Workflow

## Overview

This workflow follows the **Build Once, Deploy Many** pattern used by companies like Google, Netflix, and AWS.

### Core Principles

1. **Immutable Images**: Build containers once, never modify them
2. **Test Exact Artifacts**: Test the exact containers you'll deploy
3. **Simple Deployment**: One command to deploy from production server

## Workflow Steps

### On Dev Machine

#### Step 1: Build
```bash
./scripts/build.sh
```
- Builds production Docker images
- Tags with version: `v<commit>-<timestamp>`
- Saves version info to `_build/.build-version`

#### Step 2: Test (with Production Data)
```bash
./scripts/test.sh --load-prod-data
```
- Starts the exact containers you built
- Optionally loads production data for realistic testing
- Runs all tests: unit, integration, e2e
- Stops containers when done

#### Step 3: Push
```bash
./scripts/push.sh
```
- Pushes tested images to Docker Hub
- Tags as: `therealbogie/stg_rd:frontend-<version>`
- Only pushes if tests passed

#### All-in-One (Recommended)
```bash
./scripts/workflow.sh
```
- Runs: Build → Test → Push
- Fails fast if any step fails
- Shows version tag at end

### On Production Server

#### Deploy
```bash
./scripts/deploy.sh --version v<commit>-<timestamp>
```
- Pulls images from Docker Hub
- Stops old containers
- Starts new containers
- Runs migrations (optional)
- Verifies deployment

## Key Benefits

1. **Reproducible**: Same image everywhere
2. **Tested**: Only deploy what you've tested
3. **Simple**: One command per step
4. **Safe**: Can test before pushing
5. **Fast**: Deploy takes ~2 minutes

## Example Session

```bash
# Dev machine
./scripts/workflow.sh
# Output: Version vabc123-20260205-120000

# Production server
./scripts/deploy.sh --version vabc123-20260205-120000
```

That's it! The exact containers you tested are now running in production.
