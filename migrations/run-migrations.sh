#!/bin/bash

# STG_RD ArangoDB Migration Runner
# Usage: ./run-migrations.sh [--dry-run] [--endpoint URL] [--database DB] [--username USER] [--password PASS]

set -e

# Default values
DRY_RUN=false
ENDPOINT="${ARANGO_ENDPOINT:-http://127.0.0.1:8529}"
DATABASE="${ARANGO_DATABASE:-stg_rd}"
USERNAME="${ARANGO_USERNAME:-root}"
PASSWORD="${ARANGO_PASSWORD:-}"
# Point to new organized location for ordered migration files
MIGRATIONS_DIR="${MIGRATIONS_DIR:-./files}"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --endpoint)
            ENDPOINT="$2"
            shift 2
            ;;
        --database)
            DATABASE="$2"
            shift 2
            ;;
        --username)
            USERNAME="$2"
            shift 2
            ;;
        --password)
            PASSWORD="$2"
            shift 2
            ;;
        --migrations-dir)
            MIGRATIONS_DIR="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --dry-run              Preview changes without applying them"
            echo "  --endpoint URL         ArangoDB endpoint (default: $ENDPOINT)"
            echo "  --database DB          Database name (default: $DATABASE)"
            echo "  --username USER        Username (default: $USERNAME)"
            echo "  --password PASS        Password"
            echo "  --migrations-dir DIR   Migrations directory (default: $MIGRATIONS_DIR)"
            echo "  -h, --help            Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  ARANGO_ENDPOINT       ArangoDB endpoint"
            echo "  ARANGO_DATABASE       Database name"
            echo "  ARANGO_USERNAME       Username"
            echo "  ARANGO_PASSWORD       Password"
            echo "  MIGRATIONS_DIR        Migrations directory"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Check if password is provided
if [[ -z "$PASSWORD" ]]; then
    echo "Error: Password is required"
    echo "Provide it via --password argument or ARANGO_PASSWORD environment variable"
    exit 1
fi

# Check if migrations directory exists
if [[ ! -d "$MIGRATIONS_DIR" ]]; then
    echo "Error: Migrations directory does not exist: $MIGRATIONS_DIR"
    exit 1
fi

# Find the binary - look in multiple locations
BINARY=""
for path in "./target/release/stg-rd-migrations" "../target/release/stg-rd-migrations" "../../target/release/stg-rd-migrations"; do
    if [[ -f "$path" ]]; then
        BINARY="$path"
        break
    fi
done

if [[ -z "$BINARY" ]]; then
    echo "Error: Migration binary not found"
    echo "Tried: ./target/release/stg-rd-migrations, ../target/release/stg-rd-migrations, ../../target/release/stg-rd-migrations"
    echo ""
    echo "Build it first with one of these commands:"
    echo "  # From migrations directory:"
    echo "  cargo build --package stg-rd-migrations --release"
    echo "  # From workspace root:"
    echo "  cargo build --package stg-rd-migrations --release"
    exit 1
fi

echo "Using binary: $BINARY"

# Build command
CMD="$BINARY"
CMD="$CMD --endpoint '$ENDPOINT'"
CMD="$CMD --database '$DATABASE'"
CMD="$CMD --username '$USERNAME'"
CMD="$CMD --password '$PASSWORD'"
CMD="$CMD --migrations-dir '$MIGRATIONS_DIR'"

if [[ "$DRY_RUN" == "true" ]]; then
    CMD="$CMD --dry-run"
    echo "Running migrations in DRY-RUN mode..."
else
    echo "Running migrations..."
fi

echo "Endpoint: $ENDPOINT"
echo "Database: $DATABASE"
echo "Username: $USERNAME"
echo "Migrations dir: $MIGRATIONS_DIR"
echo ""

# Execute
eval $CMD
