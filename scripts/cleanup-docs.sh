#!/bin/bash

# Documentation cleanup script
# Removes duplicates, outdated files, and consolidates documentation

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

DRY_RUN=true
FORCE=false

usage() {
  echo "Usage: $0 [--execute] [--force]"
  echo ""
  echo "Options:"
  echo "  --execute    Actually remove files (default is dry-run)"
  echo "  --force      Skip confirmation prompts"
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --execute)
      DRY_RUN=false
      shift
      ;;
    --force)
      FORCE=true
      shift
      ;;
    --help)
      usage
      ;;
    *)
      echo "Unknown option: $1"
      usage
      ;;
  esac
done

log_info() {
  echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
  echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
  echo -e "${YELLOW}⚠️  $1${NC}"
}

remove_file() {
  local file="$1"
  local reason="$2"
  
  if [ ! -e "$file" ]; then
    log_warning "File doesn't exist: $file"
    return
  fi
  
  if [ "$DRY_RUN" = true ]; then
    log_info "Would remove: $file ($reason)"
  else
    if [ "$FORCE" != true ]; then
      read -p "Remove $file? (y/N): " -n 1 -r
      echo
      if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_warning "Skipped: $file"
        return
      fi
    fi
    
    rm -f "$file"
    log_success "Removed: $file"
  fi
}

log_info "Starting documentation cleanup..."
echo ""

if [ "$DRY_RUN" = true ]; then
  log_warning "DRY RUN MODE - No files will be removed"
  log_info "Use --execute to actually remove files"
  echo ""
fi

# 1. Cleanup plan/summary (cleanup is done)
log_info "1. Checking cleanup documentation..."
if [ -f "CLEANUP_PLAN.md" ]; then
  remove_file "CLEANUP_PLAN.md" "Cleanup is complete, no longer needed"
fi
if [ -f "CLEANUP_SUMMARY.md" ]; then
  remove_file "CLEANUP_SUMMARY.md" "Cleanup is complete, no longer needed"
fi

# 2. Duplicate TESTING.md (old one in docs root)
log_info "2. Checking for duplicate TESTING.md files..."
if [ -f "docs/TESTING.md" ] && [ -f "docs/testing/TESTING.md" ]; then
  log_warning "Found both docs/TESTING.md and docs/testing/TESTING.md"
  log_info "docs/testing/TESTING.md is the organized version"
  remove_file "docs/TESTING.md" "Duplicate, use docs/testing/TESTING.md instead"
fi

# 3. Check if TESTING_STRATEGY overlaps with TESTING_ARCHITECTURE
log_info "3. Checking for overlapping testing documentation..."
if [ -f "docs/TESTING_STRATEGY.md" ] && [ -f "docs/testing/TESTING_ARCHITECTURE.md" ]; then
  log_warning "Found both TESTING_STRATEGY.md and TESTING_ARCHITECTURE.md"
  log_info "TESTING_ARCHITECTURE.md is in organized location"
  log_info "Review if TESTING_STRATEGY.md has unique content before removing"
  if [ "$DRY_RUN" = false ]; then
    if [ "$FORCE" != true ]; then
      read -p "Remove docs/TESTING_STRATEGY.md? (y/N): " -n 1 -r
      echo
      if [[ $REPLY =~ ^[Yy]$ ]]; then
        remove_file "docs/TESTING_STRATEGY.md" "Overlaps with TESTING_ARCHITECTURE.md"
      fi
    else
      remove_file "docs/TESTING_STRATEGY.md" "Overlaps with TESTING_ARCHITECTURE.md"
    fi
  else
    log_info "Would remove: docs/TESTING_STRATEGY.md (review first)"
  fi
fi

# 4. Check ADVANCED_TESTING vs testing/ docs
log_info "4. Checking ADVANCED_TESTING.md..."
if [ -f "docs/ADVANCED_TESTING.md" ]; then
  log_warning "Found docs/ADVANCED_TESTING.md"
  log_info "Review if content is covered in docs/testing/ directory"
  log_info "Consider moving to docs/testing/ if it's testing-specific"
fi

echo ""
if [ "$DRY_RUN" = true ]; then
  log_info "Dry run complete. Review above and run with --execute to remove files."
  log_info ""
  log_info "Recommendations:"
  log_info "1. Remove CLEANUP_*.md files (cleanup is done)"
  log_info "2. Remove duplicate docs/TESTING.md (use docs/testing/TESTING.md)"
  log_info "3. Review TESTING_STRATEGY.md vs TESTING_ARCHITECTURE.md for overlap"
  log_info "4. Consider moving ADVANCED_TESTING.md to docs/testing/ if appropriate"
else
  log_success "Documentation cleanup complete!"
fi



