#!/usr/bin/env bash
set -euo pipefail

SKIP_EXTERNAL=false

for arg in "$@"; do
    case "$arg" in
        --skip-external)
            SKIP_EXTERNAL=true
            ;;
        *)
            echo "Unknown argument: $arg" >&2
            echo "Usage: $0 [--skip-external]" >&2
            exit 1
            ;;
    esac
done

run() {
    echo "=== Running: $* ==="
    "$@"
}

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

echo "=== Feature combination phase ==="

run cargo test -p stegoeggo --no-default-features
run cargo test -p stegoeggo --no-default-features --features signatures
run cargo test -p stegoeggo --no-default-features --features detached-manifest
run cargo test -p stegoeggo --no-default-features --features signatures,detached-manifest
run cargo test -p stegoeggo --all-features
run cargo test -p stegoeggo-cli --all-features

if [ "$SKIP_EXTERNAL" = false ]; then
    echo "=== External phase ==="

    run cargo test --test external_tools -- --ignored

    run cargo build --release --bin stegoeggo-conformance

    run ./target/release/stegoeggo-conformance \
        --fixtures tests/fixtures/conformance \
        --manifest tests/fixtures/conformance/manifest.toml \
        --strict \
        --json conformance-report.json
fi

echo "=== All checks passed ==="
