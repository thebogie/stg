#!/bin/bash

# Frontend runner script with environment support
# Usage: ./run_frontend.sh [development|staging|production] [--clean] [--check] [--help] [--port PORT]
clear;
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
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

print_build() {
    echo -e "${PURPLE}[BUILD]${NC} $1"
}

print_dev() {
    echo -e "${CYAN}[DEV]${NC} $1"
}

# Function to show help
show_help() {
    cat << EOF
Usage: ./run_frontend.sh [OPTIONS] [ENVIRONMENT]

ENVIRONMENT:
    development    Start in development mode (default)
    staging       Start in staging mode
    production    Start in production mode

OPTIONS:
    --clean       Clean build artifacts before running
    --check       Only check configuration without running
    --help        Show this help message
    --kill        Kill any existing frontend processes
    --restart     Restart the frontend (kill + run)
    --port PORT   Use custom port (default: 50003)
    --no-css      Skip CSS building (use existing CSS)
    --no-watch    Disable Tailwind watch mode

Examples:
    ./run_frontend.sh                    # Start in development mode
    ./run_frontend.sh production         # Start in production mode
    ./run_frontend.sh --clean           # Clean build and start in development
    ./run_frontend.sh --check production # Check production configuration
    ./run_frontend.sh --restart         # Restart the frontend
    ./run_frontend.sh --port 8080       # Start on port 8080
    ./run_frontend.sh --no-css          # Start without rebuilding CSS
EOF
}

# Function to cleanup build artifacts
cleanup_build() {
    print_build "Cleaning build artifacts..."
    cargo clean
    rm -rf dist/
    rm -rf public/styles.css
    print_build "Build artifacts cleaned"
}

# Function to check if frontend is already running
check_frontend_running() {
    local port=${FRONTEND_PORT:-50003}
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
        print_warning "Frontend is already running on port $port"
        return 0
    fi
    return 1
}

# Function to kill existing frontend processes
kill_frontend() {
    print_status "Killing existing frontend processes..."
    
    # Kill Trunk processes
    pkill -f "trunk serve" || true
    
    # Kill Tailwind processes
    pkill -f "tailwindcss.*--watch" || true
    
    # Kill processes on the frontend port
    local port=${FRONTEND_PORT:-50003}
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
        print_status "Killing process on port $port..."
        lsof -ti:$port | xargs kill -9 2>/dev/null || true
    fi
    
    sleep 2
    
    # Force kill if still running
    if pgrep -f "trunk serve" > /dev/null || pgrep -f "tailwindcss.*--watch" > /dev/null; then
        print_warning "Some processes may still be running, forcing kill..."
        pkill -9 -f "trunk serve" || true
        pkill -9 -f "tailwindcss.*--watch" || true
    fi
    
    print_status "Frontend processes killed"
}

# Function to check dependencies
check_dependencies() {
    print_header "Checking dependencies..."
    
    # Check if cargo is available
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust and Cargo first."
        exit 1
    fi
    
    # Check if trunk is available
    if ! command -v trunk &> /dev/null; then
        print_error "Trunk not found. Please install Trunk: cargo install trunk"
        exit 1
    fi
    
    # Check if npm is available
    if ! command -v npm &> /dev/null; then
        print_error "npm not found. Please install Node.js and npm first."
        exit 1
    fi
    
    # Check if node_modules exists
    if [ ! -d "node_modules" ]; then
        print_warning "node_modules not found. Installing dependencies..."
        npm install
    fi
    
    # Check if wasm32 target is installed
    if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
        print_warning "wasm32-unknown-unknown target not installed. Installing..."
        rustup target add wasm32-unknown-unknown
    fi
    
    print_status "Dependencies check passed"
}

# Function to validate environment configuration
validate_environment() {
    print_header "Validating environment configuration..."
    
    # Check if backend is accessible (optional)
    local backend_port=${BACKEND_PORT:-50002}
    if command -v curl &> /dev/null; then
        if curl -s "http://localhost:$backend_port/health" > /dev/null 2>&1; then
            print_status "Backend is accessible on port $backend_port"
        else
            print_warning "Backend may not be accessible on port $backend_port"
        fi
    fi
    
    # Check if required directories exist
    if [ ! -d "src" ]; then
        print_error "src directory not found"
        exit 1
    fi
    
    if [ ! -f "index.html" ]; then
        print_error "index.html not found"
        exit 1
    fi
    
    if [ ! -f "Trunk.toml" ]; then
        print_error "Trunk.toml not found"
        exit 1
    fi
    
    print_status "Environment validation passed"
}

# Function to build CSS
build_css() {
    if [ "$SKIP_CSS" = true ]; then
        print_warning "Skipping CSS build as requested"
        return 0
    fi
    
    print_build "Building CSS..."
    
    # Create public directory if it doesn't exist
    mkdir -p public
    
    # Build production CSS first
    if npm run build:css:prod; then
        print_build "Production CSS built successfully"
    else
        print_error "Failed to build production CSS"
        exit 1
    fi
}

# Function to start Tailwind in watch mode
start_tailwind_watch() {
    if [ "$NO_WATCH" = true ]; then
        print_warning "Tailwind watch mode disabled"
        return 0
    fi
    
    print_dev "Starting Tailwind in watch mode..."
    
    # Start Tailwind in watch mode in the background
    npm run build:css &
    TAILWIND_PID=$!
    
    # Store PID for cleanup
    echo $TAILWIND_PID > .tailwind.pid
    
    print_dev "Tailwind watch started with PID: $TAILWIND_PID"
}

# Function to setup signal handlers
setup_signal_handlers() {
    trap 'cleanup_on_exit' INT TERM EXIT
}

# Function to cleanup on exit
cleanup_on_exit() {
    print_status "Shutting down frontend..."
    
    # Kill Tailwind process if running
    if [ -f ".tailwind.pid" ]; then
        local tailwind_pid=$(cat .tailwind.pid)
        if kill -0 $tailwind_pid 2>/dev/null; then
            print_status "Stopping Tailwind watch process..."
            kill $tailwind_pid 2>/dev/null || true
        fi
        rm -f .tailwind.pid
    fi
    
    # Kill any remaining processes
    kill_frontend
    
    print_status "Frontend shutdown complete"
    exit 0
}

# Function to get build information
get_build_info() {
    print_header "Getting build information..."
    
    if [ -f "../scripts/build-info.sh" ]; then
        source ../scripts/build-info.sh
        export GIT_COMMIT
        export BUILD_DATE
        
        print_build "Build Info:"
        print_build "  Git Commit: $GIT_COMMIT"
        print_build "  Build Date: $BUILD_DATE"
    else
        print_warning "build-info.sh not found, using default values"
        export GIT_COMMIT="unknown"
        export BUILD_DATE=$(date -u +"%Y-%m-%d %H:%M:%S UTC")
    fi
}

# Parse command line arguments
ENVIRONMENT="development"
CLEAN_BUILD=false
CHECK_ONLY=false
KILL_ONLY=false
RESTART=false
SKIP_CSS=false
NO_WATCH=false
CUSTOM_PORT=""

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
        --no-css)
            SKIP_CSS=true
            shift
            ;;
        --no-watch)
            NO_WATCH=true
            shift
            ;;
        --port)
            CUSTOM_PORT="$2"
            shift 2
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
    kill_frontend
    exit 0
fi

if [ "$RESTART" = true ]; then
    kill_frontend
    # Continue with normal startup
fi

print_header "Starting frontend in $ENVIRONMENT environment..."

# Set environment variables
export RUST_ENV=${RUST_ENV:-$ENVIRONMENT}
export FRONTEND_PORT=${CUSTOM_PORT:-${FRONTEND_PORT:-50003}}
export BACKEND_PORT=${BACKEND_PORT:-50002}

# Validate environment
validate_environment

# Check dependencies
check_dependencies

# Check if frontend is already running
if check_frontend_running; then
    if [ "$ENVIRONMENT" = "production" ]; then
        print_error "Frontend is already running. Use --kill to stop it first."
        exit 1
    else
        print_warning "Frontend is already running. Consider using --restart to restart it."
    fi
fi

# Get build information
get_build_info

# Clean build if requested
if [ "$CLEAN_BUILD" = true ]; then
    cleanup_build
fi

# Check if this is just a configuration check
if [ "$CHECK_ONLY" = true ]; then
    print_status "Configuration check completed successfully"
    exit 0
fi

# Build CSS
build_css

# Setup signal handlers
setup_signal_handlers

# Start Tailwind watch mode
start_tailwind_watch

# Display configuration
print_header "Configuration Summary:"
echo "Environment: $RUST_ENV"
echo "Frontend Port: $FRONTEND_PORT"
echo "Backend Port: $BACKEND_PORT"
echo "Git Commit: $GIT_COMMIT"
echo "Build Date: $BUILD_DATE"
echo "CSS Watch Mode: $([ "$NO_WATCH" = true ] && echo "Disabled" || echo "Enabled")"
echo "Skip CSS Build: $([ "$SKIP_CSS" = true ] && echo "Yes" || echo "No")"

# Start Trunk server
print_header "Starting Trunk server..."
print_dev "Starting Trunk serve on port $FRONTEND_PORT..."

# Export build info for the build process
export GIT_COMMIT
export BUILD_DATE

# Start Trunk with proper error handling
if trunk serve --address 0.0.0.0 --port $FRONTEND_PORT --no-default-features --features frontend; then
    print_status "Frontend started successfully"
else
    print_error "Frontend failed to start"
    exit 1
fi 