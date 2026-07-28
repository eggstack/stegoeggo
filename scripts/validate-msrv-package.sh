#!/usr/bin/env bash
# validate-msrv-package.sh — Validate packaged crate compiles on MSRV with fresh resolution
#
# Packages the library, creates a clean consumer crate, removes lockfiles,
# and verifies all feature combinations compile on the declared MSRV.
#
# Exit codes:
#   0 — all checks passed
#   1 — check failed

set -euo pipefail

MSRV="${MSRV:-1.87}"
echo "=== MSRV package validation (rust ${MSRV}) ==="
echo "Rust version: $(rustc --version)"
echo ""

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

echo "=== Phase 1: Package library ==="
ALLOW_DIRTY=""
if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
    ALLOW_DIRTY="--allow-dirty"
fi
cargo package -p stegoeggo $ALLOW_DIRTY 2>&1 | tail -5
CRATE_FILE=$(find target/package -maxdepth 1 -name 'stegoeggo-0.3.2.crate' | head -1)
if [ -z "$CRATE_FILE" ]; then
    echo "ERROR: No .crate file found after packaging" >&2
    exit 1
fi
echo "Crate file: $CRATE_FILE"
echo ""

echo "=== Phase 2: Extract and prepare ==="
mkdir -p "$TMPDIR/pkg"
tar -xzf "$CRATE_FILE" -C "$TMPDIR/pkg"
PKG_DIR=$(find "$TMPDIR/pkg" -maxdepth 1 -type d -name 'stegoeggo-*' | head -1)
echo "Package dir: $PKG_DIR"
echo ""

# Feature combinations to test
FEATURE_COMBOS=(
    ""
    "async"
    "signatures"
    "detached-manifest"
    "signatures,detached-manifest"
    "async,signatures,detached-manifest"
)

for combo in "${FEATURE_COMBOS[@]}"; do
    label="${combo:-default}"
    echo "=== Phase 3: Test feature combo: ${label} ==="

    CONSUMER_DIR="$TMPDIR/consumer-${label//[,_]/-}"
    mkdir -p "$CONSUMER_DIR/src"

    FEATURES_LINE=""
    if [ -n "$combo" ]; then
        # Convert comma-separated features to TOML array syntax
        IFS=',' read -ra FEAT_ARRAY <<< "$combo"
        TOML_FEATURES=""
        for feat in "${FEAT_ARRAY[@]}"; do
            TOML_FEATURES="${TOML_FEATURES}\"${feat}\", "
        done
        FEATURES_LINE=", features = [${TOML_FEATURES%, }]"
    fi

    cat > "$CONSUMER_DIR/Cargo.toml" <<EOF
[package]
name = "stegoeggo-msrv-consumer"
version = "0.0.0"
edition = "2021"
rust-version = "${MSRV}"

[dependencies]
stegoeggo = { path = "${PKG_DIR}"${FEATURES_LINE} }
EOF

    cat > "$CONSUMER_DIR/src/lib.rs" <<'EOF'
pub fn smoke_test() {
    let notice = stegoeggo::RightsNotice::default();
    let policy = stegoeggo::RightsPolicy::Allowed;
    let _ = stegoeggo::ProtectionRequest::metadata_only(notice, policy);
}
EOF

    # Remove any lockfile
    rm -f "$CONSUMER_DIR/Cargo.lock"

    cd "$CONSUMER_DIR"
    cargo +1.87 check 2>&1
    echo "Feature combo ${label}: OK"
    echo ""
    cd - > /dev/null
done

echo "=== All MSRV package checks passed ==="
