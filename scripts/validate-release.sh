#!/usr/bin/env bash
set -euo pipefail

SKIP_EXTERNAL=false
PHASE="all"

for arg in "$@"; do
    case "$arg" in
        --skip-external)
            SKIP_EXTERNAL=true
            ;;
        --phase)
            ;;
        hermetic|external|feature|all)
            PHASE="$arg"
            ;;
        --phase=*)
            PHASE="${arg#--phase=}"
            ;;
        *)
            echo "Unknown argument: $arg" >&2
            echo "Usage: $0 [--skip-external] [--phase hermetic|external|feature|all]" >&2
            exit 1
            ;;
    esac
done

run() {
    echo "=== Running: $* ==="
    "$@"
}

run_hermetic() {
    echo "=== Hermetic phase ==="
    run cargo fmt --all -- --check
    run cargo clippy --workspace --all-targets --all-features -- -D warnings
    run cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast
    run cargo test -p stegoeggo-cli --all-features --no-fail-fast
    run cargo test --doc --workspace --exclude stegoeggo-fuzz
    run cargo package --workspace
    run cargo deny check licenses
    run cargo deny check advisories
    run cargo audit
    run cargo semver-checks check-release
    echo "=== MSRV check ==="
    run cargo +1.87 check --all-features
}

run_feature() {
    echo "=== Feature combination phase ==="
    run cargo test -p stegoeggo --no-default-features
    run cargo test -p stegoeggo --no-default-features --features signatures
    run cargo test -p stegoeggo --no-default-features --features detached-manifest
    run cargo test -p stegoeggo --no-default-features --features signatures,detached-manifest
    run cargo test -p stegoeggo --all-features
    run cargo test -p stegoeggo-cli --all-features
}

run_external() {
    echo "=== External phase ==="
    run cargo test --test external_tools -- --ignored
    run cargo build --release --bin stegoeggo-conformance
    run ./target/release/stegoeggo-conformance \
        --fixtures tests/fixtures/conformance \
        --manifest tests/fixtures/conformance/manifest.toml \
        --strict \
        --json conformance-report.json
}

case "$PHASE" in
    hermetic)
        run_hermetic
        ;;
    feature)
        run_feature
        ;;
    external)
        if [ "$SKIP_EXTERNAL" = true ]; then
            echo "External phase skipped (--skip-external)"
        else
            run_external
        fi
        ;;
    all)
        run_hermetic
        run_feature
        if [ "$SKIP_EXTERNAL" = false ]; then
            run_external
        fi
        ;;
esac

echo "=== All checks passed ==="
