#!/bin/sh
# Build the C ABI cdylib, compile the C11 test suite, and run it.
#
# Usage: bindings/c/scripts/run-c-tests.sh
# Requires: Rust toolchain, a C11 compiler (cc), the release cdylib.

set -eu

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
CRATE_DIR="$ROOT/bindings/c"
OUT_DIR="${TMPDIR:-/tmp}/stegoeggo-c-tests"
mkdir -p "$OUT_DIR"

case "$(uname -s)" in
  Darwin) LIB_NAME="libstegoeggo_c.dylib" ;;
  *) LIB_NAME="libstegoeggo_c.so" ;;
esac

# Match the cdylib target to the local C compiler (Rosetta hosts report
# x86_64 while the default Rust host target is arm64).
CARGO_TARGET=""
LIB_SUB="release"
case "$(cc -dumpmachine 2>/dev/null || echo native)" in
  x86_64-apple-darwin*) CARGO_TARGET="--target x86_64-apple-darwin"; LIB_SUB="x86_64-apple-darwin/release" ;;
esac

cargo build --manifest-path "$CRATE_DIR/Cargo.toml" --locked --release $CARGO_TARGET

case "$LIB_SUB" in
  release) LIB="$CRATE_DIR/target/release/$LIB_NAME" ;;
  *) LIB="$CRATE_DIR/target/$LIB_SUB/$LIB_NAME" ;;
esac

cc -std=c11 -Wall -Wextra -Werror "$CRATE_DIR/tests/test_c_abi.c" \
  -I"$CRATE_DIR/include" -L"$(dirname "$LIB")" -lstegoeggo_c \
  -o "$OUT_DIR/test_c_abi"
cc -std=c11 -Wall -Wextra -Werror "$CRATE_DIR/tests/c_parity_protect.c" \
  -I"$CRATE_DIR/include" -L"$(dirname "$LIB")" -lstegoeggo_c \
  -o "$OUT_DIR/c_parity_protect"
cc -std=c11 -Wall -Wextra -Werror "$CRATE_DIR/tests/c_parity_verify.c" \
  -I"$CRATE_DIR/include" -L"$(dirname "$LIB")" -lstegoeggo_c \
  -o "$OUT_DIR/c_parity_verify"

c++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only \
  -I"$CRATE_DIR/include" "$CRATE_DIR/tests/cxx_smoke.cpp"

case "$(uname -s)" in
  Darwin) export DYLD_LIBRARY_PATH="$(dirname "$LIB"):${DYLD_LIBRARY_PATH:-}" ;;
  *) export LD_LIBRARY_PATH="$(dirname "$LIB"):${LD_LIBRARY_PATH:-}" ;;
esac
"$OUT_DIR/test_c_abi"
echo "C tests OK"
