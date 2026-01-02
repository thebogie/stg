#!/bin/bash

# Cleanup script for unused files in stg_rd
# This script removes files that are no longer needed after refactoring

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

# Dry run by default
DRY_RUN=true
FORCE=false

usage() {
  echo "Usage: $0 [--execute] [--force]"
  echo ""
  echo "Options:"
  echo "  --execute    Actually remove files (default is dry-run)"
  echo "  --force      Skip confirmation prompts"
  echo "  --help       Show this help"
  exit 0
}

# Parse arguments
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

log_error() {
  echo -e "${RED}❌ $1${NC}"
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
    
    rm -rf "$file"
    log_success "Removed: $file"
  fi
}

log_info "Starting cleanup of unused files..."
echo ""

if [ "$DRY_RUN" = true ]; then
  log_warning "DRY RUN MODE - No files will be removed"
  log_info "Use --execute to actually remove files"
  echo ""
fi

# 1. Old docker script
log_info "1. Checking old docker script..."
if [ -f "docker-up.sh" ]; then
  remove_file "docker-up.sh" "Replaced by deploy/deploy.sh"
else
  log_info "   Already removed: docker-up.sh"
fi

# 2. Old testing directory
log_info "2. Checking old testing directory..."
if [ -d "testing_old" ]; then
  # Check if it's in Cargo.toml workspace
  if grep -q "testing_old" Cargo.toml 2>/dev/null; then
    log_warning "   testing_old is in Cargo.toml workspace - skipping"
  else
    remove_file "testing_old" "Replaced by testing/ directory"
  fi
else
  log_info "   Already removed: testing_old/"
fi

# 3. Accidental file
log_info "3. Checking accidental file..."
if [ -f "t --workspace" ]; then
  remove_file "t --workspace" "Appears to be accidental/typo"
else
  log_info "   Already removed: t --workspace"
fi

# 4. Old profile page
log_info "4. Checking old profile page..."
if [ -f "frontend/src/pages/profile_old.rs" ]; then
  # Check if it's referenced
  if grep -r "profile_old\|ProfileOld" frontend/src/ --exclude="profile_old.rs" 2>/dev/null | grep -v "Binary" | grep -q .; then
    log_warning "   profile_old.rs is referenced - skipping"
  else
    remove_file "frontend/src/pages/profile_old.rs" "Old profile page, not referenced"
  fi
else
  log_info "   Already removed: frontend/src/pages/profile_old.rs"
fi

# 5. Check for duplicate config script
log_info "5. Checking for duplicate setup scripts..."
if [ -f "config/setup-dev-env.sh" ] && [ -f "config/setup-env.sh" ]; then
  log_warning "   Found both setup-dev-env.sh and setup-env.sh"
  log_info "   setup-env.sh is the newer unified script"
  if [ "$DRY_RUN" = false ]; then
    if [ "$FORCE" != true ]; then
      read -p "Remove setup-dev-env.sh? (y/N): " -n 1 -r
      echo
      if [[ $REPLY =~ ^[Yy]$ ]]; then
        remove_file "config/setup-dev-env.sh" "Replaced by setup-env.sh"
      fi
    else
      remove_file "config/setup-dev-env.sh" "Replaced by setup-env.sh"
    fi
  else
    log_info "   Would remove: config/setup-dev-env.sh (replaced by setup-env.sh)"
  fi
fi

echo ""
if [ "$DRY_RUN" = true ]; then
  log_info "Dry run complete. Review above and run with --execute to remove files."
else
  log_success "Cleanup complete!"
  log_info "Removed unused files. Project is now cleaner."
fi



