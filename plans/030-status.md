# Plan 030 Status

Plan baseline SHA: 1ad9cc192460ce0efc6ce91ce25674b5f421d9c6
Candidate SHA: not yet selected (in progress)
Release version: 0.3.0
Disposition: PARTIAL

## Phase 0: Establish the baseline and release-version decision — CLOSED

### Decision

Release version chosen: **0.3.0** (major bump from 0.2.2).

Rationale: `EmbeddedReferenceStatus` gained new variants (`PresentValid`,
`AuthenticationKeyMissing`, `AuthenticationFailed`, `UnsupportedVersion`) after
the 0.2.2 publish. These variants are required for correctness (HMAC
verification, v3 payload validation). They cannot be expressed as additive
changes in a 0.x patch release.

### Changes made

1. **`TrustPolicy` restored to 0.2.2 exhaustive shape**: Removed
   `TrustVerifyingKeys` variant and `#[non_exhaustive]`. Public API matches
   published 0.2.2: `TrustNone`, `TrustKeys`, `TrustCallback`.

2. **`DetachedOverallStatus` `#[non_exhaustive]` removed**: This type did not
   exist in 0.2.2; it was introduced in 0.2.3-dev. It is now exhaustive.

3. **`EmbeddedReferenceStatus` `#[non_exhaustive]` removed**: This type existed
   in 0.2.2 but without `#[non_exhaustive]`. New variants retained (required
   for correctness). Type is now exhaustive.

4. **Additive `DetachedVerificationOptions` type**: New struct bundling
   `trust_policy`, `caller_verifying_keys`, `payload_mac_key`, and `limits`.

5. **Additive `verify_detached_manifest_with_options` function**: New entry
   point that accepts caller-owned verifying keys through the options struct
   rather than through `TrustPolicy`.

6. **CLI migrated**: `handle_verify_manifest()` now uses
   `verify_detached_manifest_with_options` with `DetachedVerificationOptions`
   instead of `TrustPolicy::TrustVerifyingKeys`.

7. **Tests migrated**: All tests in `tests/detached_manifest_tests.rs` that
   used `TrustPolicy::TrustVerifyingKeys` now use
   `verify_detached_manifest_with_options`.

8. **Version bumped**: All workspace crates updated to 0.3.0. Dependency
   constraints updated.

### Evidence

```
cargo fmt --all -- --check                           → OK
cargo clippy --workspace --all-targets --all-features -- -D warnings → OK
cargo test --workspace --exclude stegoeggo-fuzz --all-features       → 1180 passed, 27 ignored
cargo test -p stegoeggo-cli --all-features                            → 36 passed
cargo test --doc --workspace --exclude stegoeggo-fuzz                 → 14 passed
cargo test -p stegoeggo --all-features detached                       → 15 passed
cargo semver-checks check-release --baseline-version 0.2.2            → no semver update required
cargo test -p stegoeggo --no-default-features                         → 1004 passed
cargo test -p stegoeggo --no-default-features --features signatures   → 1052 passed
cargo test -p stegoeggo --no-default-features --features detached-manifest → 1025 passed
```

## Phase 1: Prefix-first v3 extraction with header validation — CLOSED

### Changes made

1. **`V3ProbeResult::V3Detected` expanded** with `header_length` field read from
   byte 3 of the v3 prefix. Validation: `header_length >= V3_CORE_SIZE` and
   `total_length >= header_length`.

2. **Non-tiled LSB extraction** (`extract_with_redundancy`): Now extracts a
   6-byte prefix first, classifies via `classify_v3_probe`, then extracts the
   exact `total_bits` for v3 payloads. Legacy V2/V1 ECC sizes are fallbacks
   only after `V3ProbeResult::NotV3`.

3. **Non-tiled LSB verification** (`verify_extract_with_redundancy`): Same
   prefix-first approach with structured `CandidateOutcome` returns for
   `MalformedV3` and `UnsupportedVersion`.

4. **Non-tiled DCT extraction** (`extract_verified_dct_payload_from_coefficients`):
   Prefix-first approach for DCT/F5 paths.

5. **Non-tiled DCT verification** (`verify_extract_dct_from_coefficients`):
   Same prefix-first approach.

6. **No remaining fixed-size v3 candidate loops** in production extraction paths.
   `V3_CRC_PAYLOAD_BITS` and `V3_HMAC_PAYLOAD_BITS` constants remain only for
   capacity estimation (`payload_bits_for_context`).

### Evidence

```
cargo test -p stegoeggo --all-features v3_  → 44 passed
cargo test -p stegoeggo --all-features legacy → 10 passed
cargo test -p stegoeggo --all-features tiled  → 29 passed
cargo test -p stegoeggo --all-features dct    → 13 passed
cargo test --workspace --all-features         → 1180 passed, 27 ignored
```

## Phase 2-7: NOT STARTED
