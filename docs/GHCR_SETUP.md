# GitHub Container Registry (GHCR) — Build on commit, pull from production

Once you run tests locally and push to `main`, GitHub Actions builds the backend Docker image and pushes it to GHCR. On your production machine you pull that image and run it with your existing stack.

## What runs on each push to `main`

**Build-and-push only** (no tests in CI; run unit, integration, and e2e locally before pushing):

- Builds the backend image from `back/api/Dockerfile.backend`, then pushes to:
   - `ghcr.io/<YOUR_GITHUB_OWNER>/<REPO_NAME>/backend:<sha>`
   - `ghcr.io/<YOUR_GITHUB_OWNER>/<REPO_NAME>/backend:latest` (only for `main`)

Example: if your repo is `myorg/stg`, the image is `ghcr.io/myorg/stg/backend:latest` (and `ghcr.io/myorg/stg/backend:<commit-sha>`).

## One-time setup (GitHub)

- **Push from Actions**: No extra setup. The workflow uses `GITHUB_TOKEN` with `packages: write`; the first push creates the package.
- **Package visibility**: By default the new package is **private**. To pull from production you either:
  - **Option A (simplest)**: Make the package public: Repo → **Packages** (right sidebar) → open the `backend` package → **Package settings** → **Change visibility** → Public, or
  - **Option B**: Keep it private and log in from production with a Personal Access Token (PAT) that has `read:packages` (see below).

## Pulling and running on production

### 1. Log in to GHCR (only if the package is private)

On the production machine:

```bash
echo "$GITHUB_PAT" | docker login ghcr.io -u YOUR_GITHUB_USERNAME --password-stdin
```

Use a PAT with `read:packages` (no `repo` scope needed for public repos; for private repos the PAT needs access to the repo or to the package).

### 2. Pull the image

```bash
# Latest build from main
docker pull ghcr.io/YOUR_OWNER/stg/backend:latest

# Or a specific commit (from the workflow run “Build and push image” → image digest or tag)
docker pull ghcr.io/YOUR_OWNER/stg/backend:SHA
```

Replace `YOUR_OWNER` with your GitHub org or username and `stg` with your repo name (lowercase).

### 3. Run with your existing stack

Your `deploy/docker-compose.yml` currently builds the backend locally (`image: stg-backend:local`). To use the GHCR image instead:

**Option A — override image at run time:**

```bash
export BACKEND_IMAGE=ghcr.io/YOUR_OWNER/stg/backend:latest
docker compose -f deploy/docker-compose.yml --env-file config/.env.prod up -d
```

Then in `deploy/docker-compose.yml` you’d use that variable for the backend service (see Option B).

**Option B — set image in compose (recommended):**

In `deploy/docker-compose.yml`, change the backend service to use the GHCR image and drop the build block when deploying from registry:

```yaml
backend:
  image: ${BACKEND_IMAGE:-ghcr.io/YOUR_OWNER/stg/backend:latest}
  # build: ... only for local dev; omit or comment out for prod
```

Then on production:

```bash
docker pull ghcr.io/YOUR_OWNER/stg/backend:latest
VOLUME_PATH=/path/to/data docker compose -f deploy/docker-compose.yml --env-file config/.env.prod up -d
```

SurrealDB and Redis stay as they are (official images); only the backend comes from GHCR.

## Optional: build on version tags

To also build and push when you push a tag (e.g. `v1.0.0`), in `.github/workflows/build-and-push.yml` uncomment the `tags: ['v*']` block under `on.push`. The same image will be tagged with the version (e.g. `backend:v1.0.0`). On production you can then:

```bash
docker pull ghcr.io/YOUR_OWNER/stg/backend:v1.0.0
```

## Build version and footer

- **Local:** `./scripts/full-prod-test.sh` builds an image tagged `stg-backend:<build_version>` (e.g. `20250312-143022-abc1234`). That version is passed as `IMAGE_TAG` into the backend container and appears in `/api/version` and in the Tauri/Yew footer.
- **GHCR:** The same Dockerfile and build-args (GIT_COMMIT, BUILD_DATE) are used. Images are tagged with commit SHA and `latest`; OCI labels include `org.opencontainers.image.version` (date-shortsha) and revision/created.
- On production, set `IMAGE_TAG` when running compose if you want the footer to show a specific tag (e.g. the SHA you pulled).

## Summary

| Step | Where | Action |
|------|--------|--------|
| 1. Test production images | Your machine | `./scripts/full-prod-test.sh` (build version → _build/<version>/) |
| 2. Commit and push | Your machine | `git push origin main` |
| 3. Build and push | GitHub Actions | Automatic: build image → push to GHCR |
| 4. Deploy | Production | `docker pull ghcr.io/OWNER/stg/backend:latest` then `docker compose ... up -d` |

No need to run `git pull` or build on the production server; you run the image CI built (after you’ve run all tests locally).
