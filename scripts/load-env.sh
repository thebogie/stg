#!/bin/bash

# Helper script to load environment variables from .env.development or .env.production
# Usage: 
#   source scripts/load-env.sh                    # Uses RUST_ENV or defaults to development
#   source scripts/load-env.sh development        # Explicitly use development
#   source scripts/load-env.sh production         # Explicitly use production

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Determine environment: use argument, RUST_ENV, or default to development
ENVIRONMENT="${1:-${RUST_ENV:-development}}"

# Normalize environment name
case "$ENVIRONMENT" in
    dev|development|Development)
        ENVIRONMENT="development"
        ;;
    prod|production|Production)
        ENVIRONMENT="production"
        ;;
    *)
        echo "Warning: Unknown environment '$ENVIRONMENT', defaulting to 'development'" >&2
        ENVIRONMENT="development"
        ;;
esac

ENV_FILE="${PROJECT_ROOT}/config/.env.${ENVIRONMENT}"

if [ ! -f "$ENV_FILE" ]; then
    echo "Error: Environment file not found: $ENV_FILE" >&2
    echo "Run: ./config/setup-env.sh $ENVIRONMENT" >&2
    return 1 2>/dev/null || exit 1
fi

# Export environment for other scripts
export RUST_ENV="${ENVIRONMENT}"

# Load environment variables, properly filtering out comments and empty lines
while IFS= read -r line; do
    # Skip empty lines and comments
    if [[ -n "$line" && ! "$line" =~ ^[[:space:]]*# ]]; then
        # Check if line contains an equals sign (valid variable assignment)
        if [[ "$line" =~ = ]]; then
            # Export the variable (handle variable substitution)
            eval "export $line"
        fi
    fi
done < "$ENV_FILE"

# Set defaults if not set (matching env templates - both use same ports)
export ARANGODB_PORT="${ARANGODB_PORT:-50011}"
export BACKEND_PORT="${BACKEND_PORT:-50012}"
export FRONTEND_PORT="${FRONTEND_PORT:-50013}"
export REDIS_PORT="${REDIS_PORT:-6379}"

# Expand variables in URLs if they contain ${VAR} syntax
if [[ "$ARANGO_URL" == *'${'* ]]; then
    export ARANGO_URL="http://localhost:${ARANGODB_PORT}"
fi

if [[ "$BACKEND_URL" == *'${'* ]]; then
    export BACKEND_URL="http://localhost:${BACKEND_PORT}"
fi

if [[ "$REDIS_URL" == *'${'* ]]; then
    export REDIS_URL="redis://localhost:${REDIS_PORT}/"
fi

# Set SERVER_PORT from BACKEND_PORT if not explicitly set
export SERVER_PORT="${SERVER_PORT:-${BACKEND_PORT}}"

