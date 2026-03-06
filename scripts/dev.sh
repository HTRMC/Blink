#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# Build wasm first
"$SCRIPT_DIR/build-wasm.sh"

# Start the dev server
cd "$ROOT_DIR/packages/blink-web"
bun run dev
