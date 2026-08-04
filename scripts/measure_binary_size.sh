#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

echo "Building stegoeggo CLI in release mode..."
cargo build --release -p stegoeggo-cli 2>&1

BINARY="target/release/stegoeggo"
if [[ ! -f "$BINARY" ]]; then
    echo "ERROR: binary not found at $BINARY" >&2
    exit 1
fi

SIZE=$(stat --format='%s' "$BINARY" 2>/dev/null || stat -f'%z' "$BINARY" 2>/dev/null)
SIZE_KB=$((SIZE / 1024))
SIZE_MB=$(echo "scale=2; $SIZE / 1048576" | bc 2>/dev/null || echo "N/A")

echo ""
echo "=== Binary Size Report ==="
echo "Binary:   $BINARY"
echo "Size:     $SIZE bytes ($SIZE_KB KB, ${SIZE_MB} MB)"
echo ""
echo "Release profile:"
grep -A6 '\[profile.release\]' Cargo.toml | tail -6
echo ""
echo "To compare against a baseline, run this script again after changes"
echo "and compare the 'Size' line. A CI gate is intentionally not added."
