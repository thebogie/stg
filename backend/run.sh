#!/bin/bash

# Backend runner script with environment support
# Usage: ./run.sh [development|staging|production] [--clean] [--check] [--help]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${BLUE}[SETUP]${NC} $1"
}

# Function to show help
show_help() {
    cat << EOF
Usage: ./run.sh [OPTIONS] [ENVIRONMENT]

ENVIRONMENT:
    development    Start in development mode (default)
    staging       Start in staging mode
    production    Start in production mode

OPTIONS:
    --clean       Clean build artifacts before running
    --check       Only check configuration without running
    --help        Show this help message
    --kill        Kill any existing backend processes
    --restart     Restart the backend (kill + run)

Examples:
    ./run.sh                    # Start in development mode
    ./run.sh production         # Start in production mode
    ./run.sh --clean           # Clean build and start in development
    ./run.sh --check production # Check production configuration
    ./run.sh --restart         # Restart the backend
EOF
}

clear

# Function to cleanup build artifacts
cleanup_build() {
    print_status "Cleaning build artifacts..."
    cargo clean
    print_status "Build artifacts cleaned"
}

# Function to check if backend is already running
check_backend_running() {
    if pgrep -f "cargo run" > /dev/null; then
        print_warning "Backend process already running"
        return 0
    fi
    return 1
}

# Function to kill existing backend processes
kill_backend() {
    print_status "Killing existing backend processes..."
    pkill -f "cargo run" || true
    sleep 2
    if pgrep -f "cargo run" > /dev/null; then
        print_warning "Some processes may still be running, forcing kill..."
        pkill -9 -f "cargo run" || true
    fi
    print_status "Backend processes killed"
}

# Function to check dependencies
check_dependencies() {
    print_header "Checking dependencies..."
    
    # Check if cargo is available
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust and Cargo first."
        exit 1
    fi
    
    # Check if required ports are available
    local backend_port=${BACKEND_PORT:-50002}
    if lsof -Pi :$backend_port -sTCP:LISTEN -t >/dev/null 2>&1; then
        print_warning "Port $backend_port is already in use"
        if [ "$ENVIRONMENT" != "production" ]; then
            print_status "Attempting to kill process using port $backend_port..."
            lsof -ti:$backend_port | xargs kill -9 2>/dev/null || true
        fi
    fi
    
    print_status "Dependencies check passed"
}

# Function to validate environment configuration
validate_environment() {
    print_header "Validating environment configuration..."
    
    local missing_vars=()
    
    # Check required variables
    local required_vars=(
        "ARANGO_URL"
        "ARANGO_DB"
        "REDIS_URL"
        "BACKEND_URL"
    )
    
    for var in "${required_vars[@]}"; do
        if [ -z "${!var}" ]; then
            missing_vars+=("$var")
        fi
    done
    
    if [ ${#missing_vars[@]} -gt 0 ]; then
        print_error "Missing required environment variables: ${missing_vars[*]}"
        exit 1
    fi
    
    # Environment-specific validation
    case $ENVIRONMENT in
        production)
            print_status "Validating production configuration..."
            
            if [ "$ARANGO_PASSWORD" = "test" ] || [ "$ARANGO_PASSWORD" = "REPLACE_WITH_SECURE_PASSWORD" ]; then
                print_error "ARANGO_PASSWORD must be set to a secure value in production!"
                exit 1
            fi
            
            if [[ "$ARANGO_URL" == *"localhost"* ]] || [[ "$ARANGO_URL" == *"127.0.0.1"* ]]; then
                print_error "ARANGO_URL cannot use localhost in production!"
                exit 1
            fi
            
            if [[ "$REDIS_URL" == *"localhost"* ]] || [[ "$REDIS_URL" == *"127.0.0.1"* ]]; then
                print_error "REDIS_URL cannot use localhost in production!"
                exit 1
            fi
            
            if [ "$RUST_LOG" = "debug" ]; then
                print_warning "Production environment should not use debug logging"
                export RUST_LOG=warn
            fi
            ;;
        development)
            print_status "Development mode - using relaxed validation"
            ;;
        staging)
            print_status "Staging mode - using production-like validation"
            ;;
    esac
    
    print_status "Environment validation passed"
}

# Function to check database connectivity
check_database_connectivity() {
    print_header "Checking database connectivity..."
    
    # Check ArangoDB
    if command -v curl &> /dev/null; then
        if curl -s "$ARANGO_URL/_api/version" > /dev/null 2>&1; then
            print_status "ArangoDB is accessible"
        else
            print_warning "ArangoDB may not be accessible at $ARANGO_URL"
        fi
    fi
    
    # Check Redis
    if command -v redis-cli &> /dev/null; then
        if redis-cli -u "$REDIS_URL" ping > /dev/null 2>&1; then
            print_status "Redis is accessible"
        else
            print_warning "Redis may not be accessible at $REDIS_URL"
        fi
    fi
}

# Function to setup signal handlers
setup_signal_handlers() {
    trap 'cleanup_on_exit' INT TERM EXIT
}

# Function to cleanup on exit
cleanup_on_exit() {
    print_status "Shutting down backend..."
    kill_backend
    print_status "Backend shutdown complete"
    exit 0
}

# Parse command line arguments
ENVIRONMENT="development"
CLEAN_BUILD=false
CHECK_ONLY=false
KILL_ONLY=false
RESTART=false

while [[ $# -gt 0 ]]; do
    case $1 in
        development|staging|production)
            ENVIRONMENT="$1"
            shift
            ;;
        --clean)
            CLEAN_BUILD=true
            shift
            ;;
        --check)
            CHECK_ONLY=true
            shift
            ;;
        --kill)
            KILL_ONLY=true
            shift
            ;;
        --restart)
            RESTART=true
            shift
            ;;
        --help|-h)
            show_help
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Handle special actions
if [ "$KILL_ONLY" = true ]; then
    kill_backend
    exit 0
fi

if [ "$RESTART" = true ]; then
    kill_backend
    # Continue with normal startup
fi

print_header "Starting backend in $ENVIRONMENT environment..."

# Check if environment file exists
ENV_FILE="../.env.$ENVIRONMENT"
if [ ! -f "$ENV_FILE" ]; then
    print_error "Environment file $ENV_FILE not found!"
    print_error "Available environments: development, production"
    exit 1
fi

# Load environment variables
print_status "Loading configuration from $ENV_FILE..."
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

# Process variable substitutions in environment variables
print_status "Processing variable substitutions..."
export REDIS_URL="redis://localhost:${REDIS_INTERNAL_PORT:-6379}/"
export BACKEND_URL="http://localhost:${BACKEND_PORT:-50002}"
export FRONTEND_URL="http://localhost:${FRONTEND_PORT:-50003}"
export ARANGO_URL="http://localhost:${ARANGODB_PORT:-50001}"

# Validate environment
validate_environment

# Check dependencies
check_dependencies

# Check database connectivity
check_database_connectivity

# Set RUST_LOG based on environment
case $ENVIRONMENT in
    development)
        export RUST_LOG=debug
        export RUST_BACKTRACE=1
        ;;
    staging)
        export RUST_LOG=info
        ;;
    production)
        export RUST_LOG=warn
        ;;
esac

# Set additional environment variables if not already set
export RUST_ENV=${RUST_ENV:-$ENVIRONMENT}
export BACKEND_PORT=${BACKEND_PORT:-50002}
export FRONTEND_PORT=${FRONTEND_PORT:-50003}
export ARANGODB_PORT=${ARANGODB_PORT:-50001}
export REDIS_PORT=${REDIS_PORT:-6379}
export REDIS_INTERNAL_PORT=${REDIS_INTERNAL_PORT:-6379}
export ARANGODB_INTERNAL_PORT=${ARANGODB_INTERNAL_PORT:-8529}
export DB_POOL_SIZE=${DB_POOL_SIZE:-5}
export REDIS_POOL_SIZE=${REDIS_POOL_SIZE:-5}
export DB_TIMEOUT=${DB_TIMEOUT:-30}
export REDIS_TIMEOUT=${REDIS_TIMEOUT:-30}
export GOOGLEMAP_API_URL=${GOOGLEMAP_API_URL:-"https://maps.googleapis.com/maps/api/place/autocomplete/json"}
export BGG_API_URL=${BGG_API_URL:-"https://api.geekdo.com/xmlapi2/search?type=boardgame&query="}
export ARANGO_USERNAME=${ARANGO_USERNAME:-root}
export ARANGO_ROOT_PASSWORD=${ARANGO_ROOT_PASSWORD:-test}
export VOLUME_PATH=${VOLUME_PATH:-"/absolute/path/to/your/volumes"}
export ENV_FILE_PATH=${ENV_FILE_PATH:-""}
export ENV_FILE=${ENV_FILE:-".env.$ENVIRONMENT"}

# Display configuration
print_header "Configuration Summary:"
echo "Environment: $RUST_ENV"
echo "Log level: $RUST_LOG"
echo "Server: $BACKEND_URL"
echo "Database: $ARANGO_DB"
echo "Redis: $REDIS_URL"
echo "Backend Port: $BACKEND_PORT"
echo "Frontend Port: $FRONTEND_PORT"
echo "ArangoDB Port: $ARANGODB_PORT"
echo "Redis Port: $REDIS_PORT"
echo "Database Pool Size: $DB_POOL_SIZE"
echo "Redis Pool Size: $REDIS_POOL_SIZE"
echo "Database Timeout: $DB_TIMEOUT"
echo "Redis Timeout: $REDIS_TIMEOUT"
echo "Google API URL: $GOOGLEMAP_API_URL"
echo "BGG API URL: $BGG_API_URL"
echo "ArangoDB Username: $ARANGO_USERNAME"
echo "ArangoDB Root Password: $ARANGO_ROOT_PASSWORD"
echo "Volume Path: $VOLUME_PATH"
echo "Environment File: $ENV_FILE"

# Check if this is just a configuration check
if [ "$CHECK_ONLY" = true ]; then
    print_status "Configuration check completed successfully"
    exit 0
fi

# Clean build if requested
if [ "$CLEAN_BUILD" = true ]; then
    cleanup_build
fi

# Check if backend is already running
if check_backend_running; then
    if [ "$ENVIRONMENT" = "production" ]; then
        print_error "Backend is already running. Use --kill to stop it first."
        exit 1
    else
        print_warning "Backend is already running. Consider using --restart to restart it."
    fi
fi

# Setup signal handlers
setup_signal_handlers

# Build and run
print_header "Building and starting backend..."
print_status "Starting cargo run in $ENVIRONMENT mode..."

# Run with proper error handling
if cargo run; then
    print_status "Backend started successfully"
else
    print_error "Backend failed to start"
    exit 1
fi 