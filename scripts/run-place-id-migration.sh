#!/bin/bash

# Venue Place ID Migration Script
# This script migrates venue place_ids using the Google Places API

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${BLUE}================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}================================${NC}"
}

# Default values
ENVIRONMENT="development"
DRY_RUN=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --env)
            ENVIRONMENT="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [--env ENVIRONMENT] [--dry-run]"
            echo ""
            echo "Options:"
            echo "  --env ENVIRONMENT    Environment to use (development/production) [default: development]"
            echo "  --dry-run            Run migration without making changes"
            echo "  --help, -h           Show this help message"
            echo ""
            echo "Environment Files:"
            echo "  The script will load variables from .env.development or .env.production"
            echo "  Required variables:"
            echo "    GOOGLE_LOCATION_API    Google Places API key"
            echo "    ARANGO_URL            ArangoDB endpoint"
            echo "    ARANGO_DB             ArangoDB database name"
            echo "    ARANGO_USERNAME       ArangoDB username"
            echo "    ARANGO_PASSWORD       ArangoDB password"
            echo ""
            echo "Example:"
            echo "  $0 --env development --dry-run"
            echo "  $0 --env production"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Load environment variables from .env file
load_environment() {
    print_info "Loading environment configuration..."
    
    # Determine environment file path
    ENV_FILE="../.env.$ENVIRONMENT"
    if [ ! -f "$ENV_FILE" ]; then
        print_error "Environment file $ENV_FILE not found!"
        print_error "Available environments: development, production"
        print_error "Please create $ENV_FILE from .env.example"
        exit 1
    fi
    
    print_info "Loading configuration from $ENV_FILE..."
    
    # Load environment variables, properly filtering out comments and empty lines
    while IFS= read -r line; do
        # Skip empty lines and comments
        if [[ -n "$line" && ! "$line" =~ ^[[:space:]]*# ]]; then
            # Check if line contains an equals sign (valid variable assignment)
            if [[ "$line" =~ = ]]; then
                # Export the variable
                export "$line"
            fi
        fi
    done < "$ENV_FILE"
    
    print_success "Environment variables loaded"
}

# Check if required environment variables are set
check_env_vars() {
    print_info "Checking environment variables..."
    
    if [ -z "$GOOGLE_LOCATION_API" ]; then
        print_error "GOOGLE_LOCATION_API environment variable is not set"
        print_error "Please add GOOGLE_LOCATION_API=your_api_key to your .env.$ENVIRONMENT file"
        exit 1
    fi
    
    # Set defaults for ArangoDB if not provided
    export ARANGO_URL=${ARANGO_URL:-"http://localhost:8529"}
    export ARANGO_DB=${ARANGO_DB:-"test"}
    export ARANGO_USERNAME=${ARANGO_USERNAME:-"test"}
    export ARANGO_PASSWORD=${ARANGO_PASSWORD:-"test"}
    
    print_info "Configuration:"
    print_info "  Environment: $ENVIRONMENT"
    print_info "  ArangoDB: $ARANGO_URL (database: $ARANGO_DB)"
    print_info "  Google API: Configured"
    
    print_success "Environment variables configured"
}

# Build the migration script
build_migration() {
    print_info "Building migration script..."
    
    cd "$(dirname "$0")"
    
    if ! cargo build --release; then
        print_error "Failed to build migration script"
        exit 1
    fi
    
    print_success "Migration script built successfully"
}

# Run the migration
run_migration() {
    print_info "Starting venue place_id migration..."
    
    # Check if dry run is requested
    DRY_RUN_FLAG=""
    if [ "$1" = "--dry-run" ]; then
        DRY_RUN_FLAG="--dry-run"
        print_warning "Running in DRY RUN mode - no changes will be made"
    fi
    
    # Run the migration
    if ! cargo run --release --bin migrate-place-ids -- $DRY_RUN_FLAG; then
        print_error "Migration failed"
        exit 1
    fi
    
    print_success "Migration completed successfully"
}

# Main execution
main() {
    print_header "🚀 Venue Place ID Migration Tool"
    
    load_environment
    check_env_vars
    build_migration
    run_migration
}

# Run the migration
run_migration() {
    print_info "Starting venue place_id migration..."
    
    if [ "$DRY_RUN" = true ]; then
        print_warning "Running in DRY RUN mode - no changes will be made"
    fi
    
    # Run the migration
    if ! cargo run --release --bin migrate-place-ids -- $([ "$DRY_RUN" = true ] && echo "--dry-run"); then
        print_error "Migration failed"
        exit 1
    fi
    
    print_success "Migration completed successfully"
}

# Execute main function
main
