# Plan 103 Status: Native Eggfetch Updater Transport and Rust 1.89 Baseline

Status: planned; not yet implemented.

## Objective

Migrate only the `stegoeggo-cli` self-updater from external `curl` to
published `eggfetch-core 0.2.x`, while preserving the current crates.io
version authority, GitHub Release asset contract, checksum/candidate
verification, narrow Cargo fallback policy, and self-replacement semantics.

The same implementation raises the repository MSRV from Rust 1.87 to Rust
1.89 because `eggfetch-core 0.2.0` requires Rust 1.89.

## Required implementation evidence

Before this status can move to complete, record:

- exact eggfetch-core version resolved in `Cargo.lock`;
- final eggfetch feature set from `cargo tree -p stegoeggo-cli -e features`;
- confirmation that root `stegoeggo` and `stegoeggo-stego` do not depend
  on eggfetch;
- exact MSRV declarations and Rust 1.89.0 validation results;
- updater loopback test results;
- `./scripts/test-release-updater.sh` result;
- cargo-deny license/advisory results;
- any behavior delta from the previous curl implementation;
- any unresolved item handed to Plan 104.

## Non-negotiable closeout checks

Do not mark complete unless:

- no updater runtime path invokes curl;
- proxy environment handling is explicit;
- connect and total deadlines are finite;
- HTTPS downgrade redirects are denied;
- response bodies are bounded;
- only exact executable-asset HTTP 404 can trigger network-path Cargo
  fallback;
- checksum, candidate identity/version, and replacement validation remain
  unchanged in strength;
- the current executable is left untouched on every pre-replacement failure.

## Next plan

After implementation, execute Plan 104 for release-target, dependency,
footprint, and end-to-end qualification.
