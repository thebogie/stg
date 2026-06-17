#!/usr/bin/env bash
# Back-compat wrapper — builds unified stg-playwright image (also tags stg-playwright-worker).
exec "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/build-playwright-image.sh" "$@"
