# Run All Tests - Single Command (ARCHIVED)

**Obsolete:** This doc references `./scripts/run-all-tests.sh`, which does not exist. Use **`./ci-local.sh all`** for the full test suite. See [../testing/HOW_TO_RUN_TESTS.md](../testing/HOW_TO_RUN_TESTS.md).

---

(Original content below kept for reference.)

## Quick Start

Run **ALL** tests with one command:

```bash
./scripts/run-all-tests.sh
```

Or using Just:

```bash
just test-everything
```

## What It Does

This script runs **EVERY** test in the project:

1. ✅ **Backend Unit Tests** - All library unit tests
2. ✅ **Backend Integration Tests** - Tests in `backend/tests/` directory (including cache tests)
3. ✅ **Testing Package Integration Tests** - Full 3-tier integration test suite
4. ✅ **Cache Integration Tests** - Explicit cache testing (if Redis available)
5. ✅ **Frontend E2E Tests** - Playwright tests (optional)

## Automatic Service Management

The script **automatically**:
- ✅ Detects if Redis/ArangoDB are running
- ✅ Starts services via `setup-hybrid-dev.sh` if needed
- ✅ Handles service cleanup

(Remainder of original doc omitted; referenced scripts and ArangoDB are no longer current.)
