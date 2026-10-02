#!/bin/sh
# Rust<->C parity: C protects and Rust verifies, then Rust protects and C verifies.
#
# Usage: bindings/c/scripts/parity.sh [fixture-png]
# Requires: Rust toolchain, a C11 compiler, the stegoeggo CLI.

set -eu

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
CRATE_DIR="$ROOT/bindings/c"
WORK="${TMPDIR:-/tmp}/stegoeggo-c-parity"
mkdir -p "$WORK"

FIXTURE="${1:-$ROOT/tests/fixtures/conformance/canonical/canonical_complete.png}"
if [ ! -f "$FIXTURE" ]; then
  echo "error: fixture not found: $FIXTURE" >&2
  exit 2
fi

# Match the cdylib target to the local C compiler (see run-c-tests.sh).
CARGO_TARGET=""
LIB_SUB="release"
case "$(cc -dumpmachine 2>/dev/null || echo native)" in
  x86_64-apple-darwin*) CARGO_TARGET="--target x86_64-apple-darwin"; LIB_SUB="x86_64-apple-darwin/release" ;;
esac
case "$(uname -s)" in
  Darwin) LIB_NAME="libstegoeggo_c.dylib" ;;
  *) LIB_NAME="libstegoeggo_c.so" ;;
esac

cargo build --manifest-path "$CRATE_DIR/Cargo.toml" --locked --release $CARGO_TARGET
cargo build -p stegoeggo-cli --locked
LIB="$CRATE_DIR/target/$LIB_SUB/$LIB_NAME"
CLI="$ROOT/target/debug/stegoeggo"

cc -std=c11 -Wall -Wextra -Werror "$CRATE_DIR/tests/c_parity_protect.c" \
  -I"$CRATE_DIR/include" -L"$(dirname "$LIB")" -lstegoeggo_c \
  -o "$WORK/c_parity_protect"
cc -std=c11 -Wall -Wextra -Werror "$CRATE_DIR/tests/c_parity_verify.c" \
  -I"$CRATE_DIR/include" -L"$(dirname "$LIB")" -lstegoeggo_c \
  -o "$WORK/c_parity_verify"

case "$(uname -s)" in
  Darwin) export DYLD_LIBRARY_PATH="$(dirname "$LIB"):${DYLD_LIBRARY_PATH:-}" ;;
  *) export LD_LIBRARY_PATH="$(dirname "$LIB"):${LD_LIBRARY_PATH:-}" ;;
esac

SEED=424242
TS="2026-01-01T00:00:00Z"

# Direction 1: C protects, Rust verifies.
"$WORK/c_parity_protect" "$FIXTURE" "$WORK/c_protected.png" "$SEED" "$TS"
"$WORK/c_parity_protect" "$FIXTURE" "$WORK/c_protected_again.png" "$SEED" "$TS"
if cmp -s "$WORK/c_protected.png" "$WORK/c_protected_again.png"; then
  echo "parity: C protect is deterministic for seed $SEED"
else
  echo "error: C protect is not deterministic" >&2
  exit 1
fi
if "$WORK/c_parity_verify" "$WORK/c_protected.png" | grep -q "status=0"; then
  echo "parity: C verifies C output (Verified)"
else
  echo "error: C does not verify its own output" >&2
  exit 1
fi
if "$CLI" verify "$WORK/c_protected.png" >/dev/null 2>&1; then
  echo "parity: Rust CLI verifies C output"
else
  echo "error: Rust CLI does not verify C output" >&2
  exit 1
fi

# Direction 2: Rust protects, C verifies.
"$CLI" protect --rights-policy prohibited-ai-ml-training --preset legal-notice-with-stego \
  --seed "$SEED" -o "$WORK/rust_protected.png" "$FIXTURE" >/dev/null 2>&1
if "$WORK/c_parity_verify" "$WORK/rust_protected.png" | grep -q "status=0"; then
  echo "parity: C verifies Rust CLI output (Verified)"
else
  echo "error: C does not verify Rust CLI output" >&2
  exit 1
fi

echo "Rust<->C parity OK"
