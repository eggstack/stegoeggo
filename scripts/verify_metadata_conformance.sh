#!/usr/bin/env bash
# verify_metadata_conformance.sh — External metadata conformance check (thin wrapper)
#
# Delegates semantic conformance checks to the Rust conformance harness.
# The harness performs internal extraction, external parser comparison,
# namespace-aware XMP validation, and cross-format semantic comparison.
#
# Usage:
#   ./scripts/verify_metadata_conformance.sh [OPTIONS]
#
# Options:
#   --strict          Fail if required tools are missing (default: skip external checks)
#   --require-complete  Fail if any required tool suite is unavailable
#   --format FMT      Filter by format (png, jpeg, webp)
#   --all-formats     Check all formats (default behavior)
#   --json PATH       Write machine-readable JSON report to PATH
#   --fixtures PATH   Path to fixtures directory (default: tests/fixtures/conformance)
#   --manifest PATH   Path to fixture manifest (default: tests/fixtures/conformance/manifest.toml)
#
# Exit codes:
#   0 — All checks passed
#   1 — Missing required tool (strict mode) or usage error
#   2 — One or more conformance checks failed

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

HARNESS=""
STRICT=""
REQUIRE_COMPLETE=""
JSON_PATH=""
FIXTURES_DIR="tests/fixtures/conformance"
MANIFEST_PATH="tests/fixtures/conformance/manifest.toml"
FORMAT_FILTER=""

while [ $# -gt 0 ]; do
    case "$1" in
        --strict) STRICT="--strict"; shift ;;
        --require-complete) REQUIRE_COMPLETE="--require-complete"; shift ;;
        --json)
            if [ -z "${2:-}" ]; then
                echo "Error: --json requires a path"
                exit 1
            fi
            JSON_PATH="$2"; shift 2 ;;
        --fixtures)
            if [ -z "${2:-}" ]; then
                echo "Error: --fixtures requires a path"
                exit 1
            fi
            FIXTURES_DIR="$2"; shift 2 ;;
        --manifest)
            if [ -z "${2:-}" ]; then
                echo "Error: --manifest requires a path"
                exit 1
            fi
            MANIFEST_PATH="$2"; shift 2 ;;
        --format)
            if [ -z "${2:-}" ]; then
                echo "Error: --format requires a format (png, jpeg, webp)"
                exit 1
            fi
            FORMAT_FILTER="--format $2"; shift 2 ;;
        --all-formats) FORMAT_FILTER=""; shift ;;
        -h|--help)
            echo "Usage: $0 [--strict] [--require-complete] [--json PATH] [--fixtures PATH] [--manifest PATH] [--format FMT] [--all-formats]"
            exit 0
            ;;
        *)
            echo "Warning: unknown argument '$1' (ignored)"
            shift
            ;;
    esac
done

find_harness() {
    if command -v stegoeggo-conformance &>/dev/null; then
        echo "stegoeggo-conformance"
    elif [ -f "target/release/stegoeggo-conformance" ]; then
        echo "target/release/stegoeggo-conformance"
    elif [ -f "target/debug/stegoeggo-conformance" ]; then
        echo "target/debug/stegoeggo-conformance"
    else
        echo ""
    fi
}

check_tool() {
    local tool="$1"
    local install_hint="$2"
    if command -v "$tool" &>/dev/null; then
        return 0
    fi
    if [ -n "$STRICT" ] || [ -n "$REQUIRE_COMPLETE" ]; then
        echo "Error: $tool is required in strict/require-complete mode but not found."
        echo "Install with: $install_hint"
        exit 2
    fi
    echo -e "${YELLOW}Warning: $tool not found. Some checks will be skipped.${NC}"
    return 0
}

check_harness() {
    HARNESS=$(find_harness)
    if [ -z "$HARNESS" ]; then
        echo "Building conformance harness..."
        if ! cargo build --release --bin stegoeggo-conformance 2>&1; then
            echo "Error: failed to build conformance harness"
            exit 2
        fi
        HARNESS="target/release/stegoeggo-conformance"
    fi
}

check_tool "exiftool" "brew install exiftool (macOS) or apt install libimage-exiftool-perl (Linux)"
check_tool "xmllint" "brew install libxml2 (macOS) or apt install libxml2-utils (Linux)"
check_tool "convert" "brew install imagemagick (macOS) or apt install imagemagick (Linux)"
check_tool "vips" "brew install vips (macOS) or apt install libvips-tools (Linux)"

if [ ! -d "$FIXTURES_DIR" ]; then
    echo -e "${YELLOW}Warning: fixtures directory not found at $FIXTURES_DIR${NC}"
    echo "No fixtures to check. Conformance suite is empty."
    exit 0
fi

check_harness

EXTRA_ARGS=""
if [ -n "$JSON_PATH" ]; then
    EXTRA_ARGS="$EXTRA_ARGS --json $JSON_PATH"
fi
if [ -f "$MANIFEST_PATH" ]; then
    EXTRA_ARGS="$EXTRA_ARGS --manifest $MANIFEST_PATH"
fi

echo "Running conformance harness..."
echo ""

set +e
$HARNESS --fixtures "$FIXTURES_DIR" $STRICT $REQUIRE_COMPLETE $FORMAT_FILTER $EXTRA_ARGS
EXIT_CODE=$?
set -e

echo ""
echo "=== Summary ==="
if [ "$EXIT_CODE" -eq 0 ]; then
    echo -e "${GREEN}All conformance checks passed.${NC}"
    exit 0
elif [ "$EXIT_CODE" -eq 1 ]; then
    echo -e "${RED}One or more conformance checks failed.${NC}"
    if [ -n "$JSON_PATH" ] && [ -f "$JSON_PATH" ]; then
        echo "JSON report: $JSON_PATH"
    fi
    exit 2
elif [ "$EXIT_CODE" -eq 3 ]; then
    echo -e "${RED}Fixture digest verification failed.${NC}"
    exit 2
elif [ "$EXIT_CODE" -eq 4 ]; then
    echo -e "${RED}Coverage minimums not met.${NC}"
    exit 2
else
    echo -e "${RED}Conformance harness encountered an error (exit code $EXIT_CODE).${NC}"
    exit 2
fi
