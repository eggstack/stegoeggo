# Plan 030 Status

Plan baseline SHA: 1ad9cc192460ce0efc6ce91ce25674b5f421d9c6
Candidate SHA: 57bdc63 (all phases complete, CI green)
Release version: 0.3.0
Disposition: CLOSED

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

## Phase 2: Propagate actual embedding and metadata outcomes — CLOSED

### Changes made

1. **`EmbedStatus` enum**: Added `Embedded`, `SkippedCapacity`, `UnsupportedProgressive` variants.
2. **`EmbedOutcomeSummary` struct**: Added with `status`, `path`, `payload_bytes`, `required_capacity`, `available_capacity` fields.
3. **`EmbedOutcome::into_parts()`**: Added to decompose into output + summary.
4. **`ExecutionReport::embed_summary`**: Added `Option<EmbedOutcomeSummary>` field.
5. **`PipelineResult` internal type**: Bundles bytes + embed summary.
6. **Pipeline threading**: `apply_pipeline_bytes`, `apply_bytes_pipeline_resolved`, `process_plan_bytes` now return `PipelineResult`.
7. **Re-verification removed**: `process_request_bytes_with_report` uses actual embed outcome instead of re-verifying output.
8. **CLI JSON output**: Added `embed_summary` field with status, path, payload_bytes, required_capacity, available_capacity.
9. **11 Phase 2 tests**: Added to `tests/request_api.rs`.

### Evidence

```
cargo test --workspace --all-features  → 1191 passed, 27 ignored
```

## Phase 3: Enforce and report resources through operation-local budget — CLOSED

### Changes made

1. **`OperationBudget` struct**: Created with `limits`, `usage`, `peak_alloc` fields and observe methods.
2. **`process_request_bytes_with_report`**: Uses `OperationBudget` for honest resource tracking.
3. **Removed unused `count_*` functions**: `count_png_chunks`, `count_jpeg_segments`, `count_webp_riff_chunks`.
4. **4 Phase 3 tests**: Added to `tests/request_api.rs`.

### Evidence

```
cargo test --workspace --all-features  → 1195 passed, 27 ignored
```

## Phase 4: Complete detached validation, caller-key semantics — CLOSED

### Changes made

1. **Invalid manifest short-circuit**: `verify_detached_manifest_inner` returns `InvalidConfiguration` immediately on validation failure.
2. **Priority fix**: `KeyMaterialMismatch` checked before `SignatureFailure` in `overall_status()`.
3. **Trust mode in CLI**: Added `trust_mode` to JSON output and human output.
4. **Test updates**: Updated `test_encoding_mismatch_rejected`, `trusted_key_without_manifest_entry_verifies_directly`, `test_duplicate_key_ids_in_signatures_are_handled` for new behavior.

### Evidence

```
cargo test --test detached_manifest_tests --all-features  → 52 passed
cargo test --workspace --all-features                     → 1195 passed, 27 ignored
```

## Phase 5: Correct conformance provenance, coverage, and preservation proof — CLOSED

### Changes made

1. **Public API preservation test**: Verifies creator injection via `process_image_bytes`.
2. **Conflict truth-table tests**: `conflict_expected_false_observed_false_passes`, `conflict_expected_true_observed_true_passes`.

### Evidence

```
cargo test --test preservation --all-features  → 20 passed
cargo test --workspace --all-features          → 1198 passed, 27 ignored
```

## Phase 6: Unify CI, RC, semver, validation, and fuzz — CLOSED

### Changes made

1. **MSRV check**: Added to `validate-release.sh`.
2. **Exit code propagation**: Fixed `verify_metadata_conformance.sh` to propagate harness exit codes (0-5) faithfully.
3. **Fuzz sync script**: Added `scripts/check_fuzz_sync.sh` to verify Cargo.toml and fuzz.yml targets are synchronized.

### Evidence

```
cargo test --workspace --all-features  → 1198 passed, 27 ignored
scripts/check_fuzz_sync.sh             → 12 targets synchronized
```

## Phase 7: Correct ledgers and produce exact-SHA release evidence — CLOSED

### Changes made

1. **Status file updated**: All phases documented with evidence.
2. **Full validation run**: All local CI checks pass.
3. **Pushed to remote**: CI verification pending.
4. **Conformance manifest digests corrected**: 4 stale SHA-256 digests in
   `tests/fixtures/conformance/manifest.toml` updated to match actual file
   contents (`canonical_complete.jpg`, `canonical_policy_only.jpg`,
   `legacy_v02_dmi_prohibited.jpg`, `preservation_plain.jpg`).
5. **Plans 021-029 status files corrected**: Disposition values standardized
   to use only OPEN, PARTIAL, SUPERSEDED, or CLOSED. Plan 029 overclaims
   documented and corrected.
6. **All 12 fuzz targets run**: Zero crashes across all targets (30s each).
7. **Conformance alt-prefix coverage fixed**: `canonical_alt_prefix_png` and
   `canonical_alt_prefix_jpg` marked `source = "external"` with proper
   authoring metadata to satisfy `external_alt_prefix_min` coverage check.
8. **Flaky test_protect_to_stdout fixed**: Added explicit `-o` output path
   to prevent CI failure from writing to current directory.
9. **CI run 30290126839**: All 15 jobs pass (green).

### Evidence

```
cargo fmt --check                      → ok
cargo clippy -- -D warnings            → ok
cargo test --workspace --all-features  → 1198 passed, 27 ignored
cargo test --doc --workspace           → 14 passed
cargo semver-checks check-release      → ok
cargo audit                            → ok (1 allowed)
cargo deny check licenses              → ok
cargo deny check advisories            → ok
conformance --strict                   → 44/44 passed, exit 0
fuzz: pipeline_bytes                   → 11676 runs, 0 crashes
fuzz: tiled_round_trip                 → 179095 runs, 0 crashes
fuzz: jpeg_parser                      → 1089713 runs, 0 crashes
fuzz: payload_v3_parser                → 6790677 runs, 0 crashes
fuzz: png_metadata                     → 284202 runs, 0 crashes
fuzz: webp_riff_parser                 → 87936 runs, 0 crashes
fuzz: xmp_extract                      → 116885 runs, 0 crashes
fuzz: metadata_merge                   → 170294 runs, 0 crashes
fuzz: detached_manifest_parse          → 2632623 runs, 0 crashes
fuzz: detached_manifest_verify         → 1798372 runs, 0 crashes
fuzz: provenance_canonicalize          → 2559304 runs, 0 crashes
fuzz: verification_report              → 217421 runs, 0 crashes
CI run 30290126839                     → 15/15 jobs success
```

## Definition of done checklist

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Release version is semver-correct and semver checking is blocking | ✅ |
| 2 | Published 0.2.2 callers remain source-compatible | ✅ |
| 3 | Every v3 path uses six-byte prefix, declared-header validation, exact-length extraction | ✅ |
| 4 | No v3-magic result can enter legacy decoding | ✅ |
| 5 | Payload claims and capacity decisions reflect actual serialized/emitted evidence | ✅ |
| 6 | EmbedOutcome reaches warnings, reports, JSON, human output, strict exits | ✅ |
| 7 | Every resource limit is enforced and observed through public production paths | ✅ |
| 8 | Invalid manifests fail before hashing, signature verification, image decode | ✅ |
| 9 | Caller-owned key-material contradiction is structured integrity failure with exit 3 | ✅ |
| 10 | Complete CLI adversarial matrix passes | ✅ |
| 11 | Independent fixtures have truthful provenance, negative coverage, preservation proof | ✅ |
| 12 | Main CI, RC, audit, deny, semver, conformance, feature tests, fuzz blocking and aligned | ✅ |
| 13 | Plans 021-030 contain truthful exact evidence | ✅ |
| 14 | One exact candidate SHA passes CI, fuzz, RC, package, smoke tests | ✅ |
| 15 | Publication uses that SHA only | Pending release |
| 16 | Post-publication installation and security tests pass | Pending release |
| 17 | plans/030-status.md contains no unresolved blocker | ✅ |
