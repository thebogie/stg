#!/bin/bash

# Clear migration lock script
# This removes the lock document that might be left from a failed migration run

set -e

# Default values
ENDPOINT="${ARANGO_ENDPOINT:-http://127.0.0.1:50001}"
DATABASE="${ARANGO_DATABASE:-smacktalk}"
USERNAME="${ARANGO_USERNAME:-root}"
PASSWORD="${ARANGO_PASSWORD:-}"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
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
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --endpoint URL         ArangoDB endpoint (default: $ENDPOINT)"
            echo "  --database DB          Database name (default: $DATABASE)"
            echo "  --username USER        Username (default: $USERNAME)"
            echo "  --password PASS        Password"
            echo "  -h, --help            Show this help message"
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

echo "Clearing migration lock..."
echo "Endpoint: $ENDPOINT"
echo "Database: $DATABASE"
echo "Username: $USERNAME"

# Use curl to directly remove the lock document
curl -X DELETE \
  -H "Authorization: Bearer $(curl -s -X POST $ENDPOINT/_open/auth \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"$USERNAME\",\"password\":\"$PASSWORD\"}" | jq -r '.jwt')" \
  "$ENDPOINT/_db/$DATABASE/_api/document/migration_lock/lock"

echo ""
echo "Lock cleared. You can now run migrations again."
