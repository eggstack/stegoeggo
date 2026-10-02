#!/bin/sh
# Audit dynamic `stegoeggo_*` exports of the built cdylib against the manifest.
#
# Usage: bindings/c/scripts/check-exports.sh [path-to-library]
# Defaults to the local release artifact for the host platform.

set -eu

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
CRATE_DIR="$ROOT/bindings/c"
MANIFEST="$CRATE_DIR/abi-v1-symbols.txt"

case "$(uname -s)" in
  Darwin) DEFAULT_LIB="$CRATE_DIR/target/release/libstegoeggo_c.dylib" ;;
  *) DEFAULT_LIB="$CRATE_DIR/target/release/libstegoeggo_c.so" ;;
esac
LIB="${1:-$DEFAULT_LIB}"

if [ ! -f "$LIB" ]; then
  echo "error: library not found: $LIB (build it first)" >&2
  exit 2
fi

EXPORTS="$(mktemp "${TMPDIR:-/tmp}/stegoeggo-exports.XXXXXX")"
trap 'rm -f "$EXPORTS"' EXIT INT TERM

case "$(uname -s)" in
  Darwin)
    nm -gU "$LIB" | awk '{print $NF}' | grep '^_stegoeggo_' | sed 's/^_//' | LC_ALL=C sort -u > "$EXPORTS"
    ;;
  *)
    nm -D --defined-only "$LIB" | awk '{print $NF}' | grep '^stegoeggo_' | grep -v '@' | LC_ALL=C sort -u > "$EXPORTS"
    ;;
esac

COUNT="$(wc -l < "$EXPORTS" | tr -d ' ')"
if [ "$COUNT" != "90" ]; then
  echo "error: $COUNT stegoeggo_* exports in $LIB, expected 90" >&2
  exit 1
fi

if cmp -s "$EXPORTS" "$MANIFEST"; then
  echo "export audit OK: $LIB exports exactly the 90-symbol manifest"
else
  echo "error: exports of $LIB differ from $MANIFEST" >&2
  diff "$MANIFEST" "$EXPORTS" >&2 || true
  exit 1
fi
