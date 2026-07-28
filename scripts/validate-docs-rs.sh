#!/usr/bin/env bash
# validate-docs-rs.sh — Docs.rs-equivalent rustdoc validation
#
# Reproduces docs.rs build conditions:
# nightly toolchain, DOCS_RS=1, cfg(docsrs), all features.
#
# Run when changing cfg(docsrs), documentation attributes, docs.rs metadata,
# feature-gated public API docs, or package inclusion.
#
# Prerequisites: nightly Rust toolchain
#
# Exit codes:
#   0 — all checks passed
#   1 — check failed

set -euo pipefail

check_dirty="${CHECK_DIRTY:-false}"

if [ "$check_dirty" = "true" ]; then
    if [ -n "$(git status --porcelain)" ]; then
        echo "ERROR: Working tree is dirty" >&2
        git status --porcelain
        exit 1
    fi
fi

VERSION=$(cargo metadata --no-deps --format-version 1 2>/dev/null \
    | python3 -c "import json,sys; print(json.load(sys.stdin)['packages'][0]['version'])")
echo "=== Docs.rs-equivalent validation (stegoeggo ${VERSION}) ==="
echo "Rust version: $(rustc +nightly --version)"
echo ""

export DOCS_RS=1
export RUSTDOCFLAGS="--cfg docsrs --check-cfg=cfg(docsrs) -D warnings"

echo "=== Phase 1: Workspace docs (nightly, DOCS_RS=1, all features) ==="
RUSTDOCFLAGS="--cfg docsrs --check-cfg=cfg(docsrs) -D warnings" \
  cargo +nightly rustdoc \
    -p stegoeggo \
    --all-features \
    --lib \
    2>&1
echo "Workspace docs: OK"

echo ""
echo "=== Phase 2: Reject module-level feature(doc_cfg) ==="
if grep -rn 'feature(doc_cfg)' src/ | grep -v 'src/lib.rs' | grep -v '//.*feature(doc_cfg)'; then
    echo "ERROR: feature(doc_cfg) found outside crate root" >&2
    grep -rn 'feature(doc_cfg)' src/ | grep -v 'src/lib.rs' | grep -v '//.*feature(doc_cfg)' >&2
    exit 1
fi
echo "No disallowed feature(doc_cfg) placement: OK"

echo ""
echo "=== Phase 3: Packaged crate docs ==="
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

ALLOW_DIRTY=""
if [ "$check_dirty" = "false" ]; then
    ALLOW_DIRTY="--allow-dirty"
fi

cargo package -p stegoeggo $ALLOW_DIRTY 2>&1 | tail -5
CRATE_FILE=$(find target/package -maxdepth 1 -name "stegoeggo-${VERSION}.crate" | head -1)
if [ -z "$CRATE_FILE" ]; then
    echo "ERROR: No .crate file found after packaging" >&2
    exit 1
fi
echo "Crate file: $CRATE_FILE"

mkdir -p "$TMPDIR/pkg"
tar -xzf "$CRATE_FILE" -C "$TMPDIR/pkg"

PKG_DIR=$(find "$TMPDIR/pkg" -maxdepth 1 -type d -name 'stegoeggo-*' | head -1)
if [ -z "$PKG_DIR" ]; then
    echo "ERROR: No extracted package directory found" >&2
    exit 1
fi

echo "Package dir: $PKG_DIR"

for f in src/lib.rs Cargo.toml; do
    if [ ! -f "$PKG_DIR/$f" ]; then
        echo "ERROR: Package missing $f" >&2
        exit 1
    fi
done
echo "Package contains required files: OK"

cd "$PKG_DIR"
export DOCS_RS=1
export RUSTDOCFLAGS="--cfg docsrs --check-cfg=cfg(docsrs) -D warnings"
cargo +nightly rustdoc \
    --all-features \
    --lib \
    2>&1
echo "Packaged crate docs: OK"

echo ""
echo "=== Phase 4: Stable docs without cfg(docsrs) still work ==="
cd - > /dev/null
unset DOCS_RS
unset RUSTDOCFLAGS
cargo doc --lib 2>&1
echo "Stable docs (no cfg(docsrs)): OK"

echo ""
echo "=== All docs.rs-equivalent checks passed ==="
