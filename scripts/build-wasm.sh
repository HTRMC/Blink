#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

echo "Building blink-core to WebAssembly..."

# Check for wasm-pack
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Install it with:"
    echo "  cargo install wasm-pack"
    exit 1
fi

# Build wasm
cd "$ROOT_DIR/crates/blink-core"
wasm-pack build --target web --out-dir "$ROOT_DIR/packages/blink-web/wasm" --out-name blink_core

# Clean up unnecessary files
rm -f "$ROOT_DIR/packages/blink-web/wasm/.gitignore"
rm -f "$ROOT_DIR/packages/blink-web/wasm/package.json"

echo "WASM build complete! Output: packages/blink-web/wasm/"
