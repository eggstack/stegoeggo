#!/bin/sh
# Generate (or verify) the committed C header from the Rust C ABI crate.
#
# Usage:
#   bindings/c/scripts/generate-header.sh          # regenerate in place
#   bindings/c/scripts/generate-header.sh --check  # fail on drift
#
# Requires: cbindgen 0.29.4 on PATH.

set -eu

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
CRATE_DIR="$ROOT/bindings/c"
OUT="$CRATE_DIR/include/stegoeggo.h"
TMP="$(mktemp "${TMPDIR:-/tmp}/stegoeggo-h.XXXXXX")"
HDR_SYMS="$(mktemp "${TMPDIR:-/tmp}/stegoeggo-syms.XXXXXX")"
trap 'rm -f "$TMP" "$HDR_SYMS"' EXIT INT TERM

CBINDGEN_VERSION="$(cbindgen --version 2>/dev/null || true)"
case "$CBINDGEN_VERSION" in
  "cbindgen 0.29.4") ;;
  *)
    echo "error: cbindgen 0.29.4 required (found: ${CBINDGEN_VERSION:-missing})" >&2
    exit 2
    ;;
esac

cbindgen --config "$CRATE_DIR/cbindgen.toml" --crate stegoeggo-c "$CRATE_DIR" --output "$TMP"

if [ ! -s "$TMP" ]; then
  echo "error: cbindgen produced empty output" >&2
  exit 2
fi

grep -o 'stegoeggo_[a-z0-9_]*(' "$TMP" | tr -d '(' | LC_ALL=C sort -u > "$HDR_SYMS"
if ! cmp -s "$HDR_SYMS" "$CRATE_DIR/abi-v1-symbols.txt"; then
  echo "error: generated header exports differ from abi-v1-symbols.txt" >&2
  diff "$CRATE_DIR/abi-v1-symbols.txt" "$HDR_SYMS" >&2 || true
  exit 1
fi
FUNCS="$(wc -l < "$HDR_SYMS" | tr -d ' ')"
if [ "$FUNCS" != "90" ]; then
  echo "error: generated header declares $FUNCS functions, expected 90" >&2
  exit 1
fi

if grep -q 'struct stegoeggo_v1_.*{' "$TMP"; then
  echo "error: generated header leaks an opaque handle body" >&2
  exit 1
fi

if [ "${1:-}" = "--check" ]; then
  if [ ! -f "$OUT" ]; then
    echo "error: committed header $OUT missing" >&2
    exit 1
  fi
  if cmp -s "$TMP" "$OUT"; then
    echo "header drift check OK: $OUT matches cbindgen 0.29.4 output"
  else
    echo "error: header drift: $OUT differs from cbindgen 0.29.4 output" >&2
    exit 1
  fi
else
  mkdir -p "$(dirname "$OUT")"
  mv "$TMP" "$OUT"
  trap 'rm -f "$HDR_SYMS"' EXIT INT TERM
  echo "generated $OUT"
fi

rm -f "$HDR_SYMS"
trap - EXIT INT TERM

python3 "$CRATE_DIR/scripts/check-contract.py"
