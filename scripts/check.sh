#!/usr/bin/env bash
# Fast deterministic checks used by local development and required CI.
# Do not add external, release, network-audit, fuzz, benchmark, or publication steps here.
set -euo pipefail

run() {
    printf '\n==> '
    printf '%q ' "$@"
    printf '\n'
    "$@"
}

run cargo fmt --all -- --check
run cargo clippy --workspace --all-targets --all-features -- -D warnings
run cargo check -p stegoeggo --no-default-features
run cargo test --workspace --exclude stegoeggo-fuzz --all-features
