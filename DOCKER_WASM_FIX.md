# Docker + WASM Fix: Industry Standard Approach

## The Problem

Your current Dockerfile is **fighting Docker's caching** instead of using it:
- Deleting everything defeats Docker layer caching
- Random build IDs prevent caching
- Too complex (500+ lines)
- Still getting stale WASM

## Industry Standard Solution

### Key Principles

1. **Let Docker cache dependencies** - Don't delete everything
2. **Copy dependencies BEFORE source** - So source changes don't rebuild deps
3. **Trust Trunk's filehash** - Trunk already handles cache busting
4. **Simple multi-stage build** - Build stage → Runtime stage
5. **Use BuildKit cache mounts** (optional, for speed)

### What Others Do

**Standard pattern:**
```dockerfile
# 1. Install tools (cached forever)
RUN cargo install trunk

# 2. Copy dependency files (cached if unchanged)
COPY Cargo.toml package.json

# 3. Install/build dependencies (cached if deps unchanged)
RUN npm ci && cargo build --dependencies-only

# 4. Copy source code LAST (invalidates when source changes)
COPY src ./src

# 5. Build application (only rebuilds if source changed)
RUN trunk build
```

### Your Current Problem

Your Dockerfile:
- Copies source early → deps rebuild every time
- Deletes everything → defeats caching
- Too many cache-busting tricks → complexity

### The Fix

See `Dockerfile.frontend.clean` - a clean, industry-standard version:

**Key differences:**
1. ✅ Dependencies built separately (cached)
2. ✅ Source copied last (only rebuilds when changed)
3. ✅ No cache deletion (let Docker cache)
4. ✅ Trust Trunk's filehash (it works!)
5. ✅ Simple verification (just check WASM exists)

### How to Use

```bash
# Test the clean Dockerfile
docker build -f frontend/Dockerfile.frontend.clean \
  --build-arg BUILD_DATE="$(date -u +%Y-%m-%d\ %H:%M:%S\ UTC)" \
  --build-arg GIT_COMMIT="$(git rev-parse --short HEAD)" \
  -t test-frontend:clean \
  .

# If it works, replace the old one
mv frontend/Dockerfile.frontend frontend/Dockerfile.frontend.old
mv frontend/Dockerfile.frontend.clean frontend/Dockerfile.frontend
```

### Why This Works

1. **Docker caches layers** - Dependencies layer cached, only rebuilds when deps change
2. **Source layer invalidates** - When you change source, only that layer rebuilds
3. **Trunk handles cache busting** - filehash=true creates unique filenames
4. **Simple = reliable** - Less code = fewer bugs

### Optional: BuildKit Cache Mounts (Faster)

For even faster builds, use BuildKit cache mounts:

```dockerfile
# Cache Cargo registry
RUN --mount=type=cache,target=/root/.cargo/registry \
    cargo build --dependencies-only

# Cache Trunk output
RUN --mount=type=cache,target=/app/frontend/.stage \
    trunk build
```

But the clean Dockerfile should work fine without this.
