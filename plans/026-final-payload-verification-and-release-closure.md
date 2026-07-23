# Plan 026: Final Payload, Verification, and Release Closure

## Status

Ready for implementation. This is a narrowly scoped closure pass. Do not tag or publish another release until every blocking acceptance criterion in this plan is satisfied and recorded in `plans/026-status.md`.

## Audited baseline

This plan was written against `main` at commit:

- `6a7ccea45d0c8e9f93f7d60a5066e682bed0e509`

The repository has substantially implemented Plans 021–025. Real Ed25519 signing, payload-v3 writing, stricter policy resolution, detached-manifest verification, resource-limit types, expanded fuzzing, and safer release workflows are present. The remaining defects are integration and evidence defects, not a reason to redesign the product.

## Objective

Close the remaining mismatch between the documented product contract and executable behavior, then produce one clean patch release after the already-published `0.2.2` artifacts.

At completion:

1. Every hidden-marker extraction path reads the exact payload-v3 length and retains v1/v2 compatibility.
2. Payload capacity calculations and embedded channel flags describe the payload actually emitted.
3. Detached-manifest CLI operations use the same bounded, trust-aware, binding-aware verifier as the library.
4. Embedded references are verified from raw image bytes, including payload version, digest, and authentication state.
5. Resource limits apply before work on every top-level processing path, and execution reports contain observed rather than placeholder usage.
6. CI and release-candidate validation exercise the library, CLI, feature combinations, conformance suite, and fuzz smoke targets intentionally.
7. Source-aware conformance minima cannot be satisfied by setting independent legacy/conflict/preservation requirements to zero.
8. Plans 021–026 have truthful status ledgers with commit, test, CI, artifact, and residual-risk evidence.
9. A new immutable patch version is validated and published. The existing `v0.2.2` tag and crates must not be altered.

## Scope guardrails

This pass must not:

- Add another image format.
- Add another signature algorithm.
- Add C2PA, certificate authorities, hosted trust services, blockchains, or network registries.
- Replace the current steganographic algorithms.
- Remove v1 or v2 payload readers.
- remove deprecated APIs outside a separately approved semver-major release.
- Redesign the CLI into a new command hierarchy unless required to correct documentation. Prefer correcting documentation to match the existing positional CLI.
- Add broad performance optimization unrelated to the acceptance criteria.
- Move or recreate the `v0.2.2` tag.
- Publish corrected code under version `0.2.2`.

Any implementation commit containing unrelated refactors should be split before review.

## Primary affected areas

Expected work is concentrated in:

- `src/protected/steganography.rs`
- `src/payload_v3/**`
- `src/lib.rs`
- `src/types.rs`
- `src/resource_limits.rs`
- `src/protected/metadata_trap.rs`
- `src/protected/notice_verification.rs`
- `src/jpeg_transcoder/**`
- `src/detached/manifest.rs`
- `src/detached/verify.rs`
- `stegoeggo-cli/src/main.rs`
- `stegoeggo-cli/tests/**`
- `tests/payload_v3_roundtrip.rs`
- `tests/independent_v3_parser.rs`
- `tests/request_api.rs`
- `tests/detached_manifest_tests.rs`
- `tests/resource_limits_tests.rs` or equivalent focused test file
- `tests/external_tools.rs`
- `tests/fixtures/conformance/**`
- `.github/workflows/ci.yml`
- `.github/workflows/release-candidate.yml`
- `.github/workflows/publish.yml`
- `.github/workflows/fuzz.yml`
- `scripts/validate-release.sh`
- `README.md`
- `SUPPORT.md`
- `STABILITY.md`
- `SECURITY.md`
- `CHANGELOG.md`
- `plans/021-status.md`
- `plans/022-status.md`
- `plans/023-status.md`
- `plans/024-status.md`
- `plans/025-status.md`
- `plans/026-status.md`

## Required implementation order

Perform the work in this order:

1. Payload-v3 extraction and capacity correctness.
2. Detached-manifest library and CLI convergence.
3. Resource-limit and usage-accounting closure.
4. CI, documentation, and independent conformance closure.
5. Status ledgers, release candidate, version bump, and publication.

Do not begin publication work while Gates 1–4 have failing tests or unresolved review findings.

---

## Gate 1: Make payload-v3 extraction and reporting exact

### 1.1 Replace legacy-size probing with a version-aware probe

Current production writing emits variable-length v3 payloads, but several LSB, tiled, and DCT extraction loops still request fixed v1/v2-sized bit windows. Replace this with one shared version-aware extraction procedure.

The preferred algorithm is:

1. Extract enough bits to inspect the v3 core header (`V3_CORE_SIZE`).
2. If the magic and version identify v3:
   - Validate header length, total length, authentication algorithm, authentication tag length, key-ID length, extension lengths, and `ResourceLimits::max_payload_bytes`.
   - Re-extract exactly `total_length * 8` bits from the same channel, seed, redundancy, tile, or coefficient set.
   - Verify the declared authentication mode.
   - Return a structured v3 result.
3. If the v3 magic is present but the header is malformed, return a malformed/invalid v3 result. Do not reinterpret those bytes as v2 or v1.
4. Only when v3 magic is absent, try the frozen v2 and v1 candidate sizes.

Apply the same procedure to:

- Non-tiled PNG/WebP LSB extraction.
- Tiled PNG/WebP LSB extraction.
- Baseline JPEG DCT/F5 extraction.
- Tiled JPEG DCT/F5 extraction.
- Verification variants that distinguish valid, invalid, malformed, missing key, and not found.

#### Difficult-area example

Use one internal probe/result model instead of duplicating size loops:

```rust
struct PayloadProbe {
    version: u8,
    total_bytes: usize,
    auth: ObservedPayloadAuth,
}

enum PayloadCandidateOutcome {
    Valid { bytes: Vec<u8>, probe: PayloadProbe },
    MissingAuthenticationKey { probe: PayloadProbe },
    InvalidIntegrity { probe: Option<PayloadProbe> },
    MalformedV3,
    UnsupportedVersion(u8),
    NotFound,
}
```

The exact names may differ. The important property is that all embedding channels call one parser/probe contract and do not maintain independent lists such as only `[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS]`.

### 1.2 Derive capacity from the actual payload bytes

Delete v2-based capacity calculations from current-write paths.

For each attempted embed:

```text
required_bits = generated_payload.len() * 8 * effective_redundancy
```

Use the exact generated payload for:

- LSB pixel requirements.
- DCT coefficient requirements.
- Tiled per-tile requirements.
- Warning/report fields.

Legacy constants may remain only in legacy-read compatibility code and compatibility tests.

### 1.3 Derive v3 channel flags from emitted evidence

Do not hard-code all channel flags to `true`.

The v3 header must reflect actual output:

- `rights_metadata`: true only when the processing plan emits the canonical metadata channel.
- `hidden_marker`: true because the payload writer is invoked only for a hidden-marker attempt that is actually being embedded.
- `authentication`: true only for a cryptographic authentication mode such as HMAC or signature. CRC32 integrity alone is not authentication.
- Tiled/progressive flags: derived from the actual selected execution path.

#### Example

For a metadata-plus-hidden-marker request using CRC32:

```text
rights_metadata = true
hidden_marker   = true
authentication = false
auth_algorithm = crc32
```

For an HMAC request:

```text
rights_metadata = true
hidden_marker   = true
authentication = true
auth_algorithm = hmac_sha256_truncated
auth_tag_len   = 16
```

### 1.4 Report actual embedding outcomes

Embedding helpers must return a structured outcome rather than silently returning unchanged bytes when capacity is insufficient.

A minimal internal model may be:

```rust
enum EmbedOutcome<T> {
    Embedded {
        output: T,
        payload_bytes: usize,
        required_capacity: usize,
        available_capacity: usize,
    },
    SkippedCapacity {
        output: T,
        required_capacity: usize,
        available_capacity: usize,
    },
    UnsupportedProgressive {
        output: T,
    },
}
```

`BestEffort` may still produce degraded output because degradation is explicit in that mode, but:

- It must emit a deterministic warning.
- `ExecutionReport.stego_succeeded` must be false.
- The report must not claim payload version 3 was emitted.
- Strict CLI mode must fail when the corresponding warning is error-severity for the selected contract.

### 1.5 Payload tests

Add focused tests for:

- V3 CRC payload exact-length extraction in PNG, WebP, baseline JPEG, tiled LSB, and tiled JPEG.
- V3 HMAC payload exact-length extraction in the same supported channels.
- A carrier that fits v3 CRC but does not fit the larger legacy v2 ECC probe still verifies successfully.
- A carrier that fits v3 HMAC but not a legacy fixed window still verifies successfully.
- Malformed v3 magic/header cannot fall through and become valid v2/v1.
- Missing HMAC key is distinct from corruption and not-found.
- Wrong HMAC key is distinct from missing key.
- V1 and v2 checked-in compatibility fixtures still read.
- Capacity warnings use v3 byte lengths.
- CRC payload channel flags do not claim authentication.
- HMAC payload channel flags do claim authentication.
- Tiny image and insufficient DCT-capacity cases never report `stego_succeeded = true`.

### Gate 1 acceptance criteria

- No current-write or capacity path uses v2 payload-size constants.
- Every LSB and DCT extraction path probes v3 first and re-extracts its declared exact length.
- Malformed v3 never falls back to legacy parsing.
- All new output verifies as v3 through the normal public byte API.
- V1/v2 fixtures remain readable.
- Embedded channel flags match actual emitted evidence.
- Degraded capacity cases are observable in warnings and reports.
- All Gate 1 tests pass under `cargo test -p stegoeggo --all-features`.

---

## Gate 2: Converge detached-manifest CLI and library verification

### 2.1 Add one bounded bytes-to-verdict entrypoint

Provide one public or crate-internal entrypoint used by the CLI that:

1. Accepts raw manifest bytes.
2. Applies `DetachedManifest::from_json_with_limits` or an equivalent bounded parser.
3. Validates schema, algorithms, key/signature encodings, duplicate identities, counts, and lengths.
4. Accepts raw image bytes.
5. Evaluates image bindings, signatures, caller trust, and optional embedded reference.
6. Returns one structured verification verdict.

Avoid a CLI-only copy of digest and signature logic.

A suitable shape is:

```rust
pub fn verify_detached_manifest_bytes(
    image_bytes: &[u8],
    manifest_bytes: &[u8],
    trust: &TrustPolicy,
    options: &DetachedVerificationOptions,
    limits: &ResourceLimits,
) -> Result<ManifestVerification>;
```

The exact API may differ, but the CLI must call the same parser and verifier used by library consumers.

### 2.2 Verify embedded references from raw bytes

Do not decode to `DynamicImage` before extracting an embedded reference. Use the raw byte API so JPEG quantization tables, image metadata seeds, and exact embedded bytes remain available.

The embedded-reference verifier must compare:

- Declared payload version.
- SHA-256 digest of the exact raw embedded payload bytes.
- Structural validity.
- Integrity/authentication result.

If the payload declares HMAC and no payload MAC key is supplied, return a distinct status such as `AuthenticationKeyMissing`; do not report `Present`.

#### Difficult-area example

```rust
enum EmbeddedReferenceStatus {
    NotProvided,
    Stripped,
    Malformed,
    VersionMismatch,
    DigestMismatch,
    AuthenticationKeyMissing,
    AuthenticationFailed,
    PresentValid,
}
```

A valid signature over a manifest does not make an invalid embedded reference valid.

### 2.3 Define trust and key CLI semantics

Keep public-key trust separate from payload HMAC authentication.

Recommended CLI semantics:

- `--trust-key <path>`: caller-supplied Ed25519 public key trusted for manifest-signature evaluation.
- `--payload-key <hex|@path|-|env>`: optional HMAC key used only for embedded-payload verification.
- No `--trust-key`: signatures may be cryptographically valid, but the overall trust result is untrusted.
- Manifest-provided `trust_metadata` remains informational and cannot set trust.

The existing option names may be retained for compatibility, but their help text and behavior must be unambiguous.

### 2.4 Route `sign` through bounded parsing

The CLI `sign` command must:

- Parse through the bounded manifest parser.
- Reject unsupported schema versions and malformed records.
- Reject duplicate key/signature identities that create ambiguity.
- Reject adding a duplicate signature for the same key unless an explicit replacement operation is implemented.
- Write atomically rather than overwriting the manifest in place before successful serialization.

### 2.5 Route `verify-manifest` through the structured verifier

Delete the manual CLI verification implementation or reduce it to formatting the library result.

Human and JSON output must derive from the same verdict object.

The verdict must include at least:

- Manifest/schema validity.
- Instance digest validity.
- Format validity.
- Dimension validity.
- File-size validity.
- Signature structural validity.
- Signature cryptographic validity.
- Key-ID match.
- Caller trust result.
- Embedded-reference status.
- Overall result.

### 2.6 Stable exit behavior

For `verify-manifest`, use documented and tested mappings:

- `0`: all required bindings valid, at least one required signature cryptographically valid and trusted, and any declared embedded reference is valid.
- `2`: malformed command input, malformed manifest, unsupported schema/algorithm, or invalid key encoding.
- `3`: digest, binding, signature, payload-integrity, or embedded-reference failure.
- `4`: cryptographically valid evidence exists but no valid signature is trusted by caller policy.
- `5`: unexpected internal failure.

Do not exit `0` after printing `MISMATCH`, `INVALID`, `SKIPPED`, or an untrusted overall result.

### 2.7 Detached-verification tests

Add CLI and library tests for:

- Oversized manifest rejected before verification.
- Unknown schema and algorithm rejected.
- Duplicate key IDs and duplicate signature IDs rejected.
- Invalid hex and wrong key length rejected.
- Correct signature and trusted key succeeds.
- Correct signature without caller trust exits `4`.
- Wrong key, modified image, wrong format claim, wrong dimensions, and wrong file size exit `3`.
- Manifest `trust_metadata.trusted = true` cannot produce trust.
- Embedded v3 CRC reference succeeds.
- Embedded v3 HMAC reference without payload key reports missing key and exits `3`.
- Embedded v3 HMAC reference with the correct payload key succeeds.
- Wrong embedded payload version and digest fail independently.
- JPEG embedded-reference verification succeeds through the raw-byte path.
- Human and JSON results agree on the overall outcome.

### Gate 2 acceptance criteria

- No CLI command deserializes detached manifests through raw `serde_json::from_slice`.
- `verify-manifest` contains no independent digest/signature verifier.
- All detached CLI output is produced from the library verdict.
- Caller trust and manifest assertions remain separate.
- Embedded references use raw-byte extraction and exact digest/version/authentication checks.
- Failure exit codes are covered by integration tests.
- All Gate 2 tests pass under the signatures-plus-detached feature combination.

---

## Gate 3: Close resource-limit and usage-accounting bypasses

### 3.1 Enforce input size before every processing branch

Move `max_input_bytes` enforcement before the metadata-only early return and before any decode, copy, hash, or metadata traversal.

This applies to:

- Request API processing.
- Legacy byte processing.
- Metadata-only processing.
- Verification APIs.
- Detached-manifest image verification.
- CLI file reads where a bounded streaming pre-check is practical; the library check remains mandatory.

### 3.2 Enforce default dimensions unconditionally

`ResourceLimits.max_width` and `max_height` must apply whether or not `ProcessingOptions.max_dimension` is set.

If both limits are present:

```text
effective_max_width  = min(resource max_width, processing max_dimension)
effective_max_height = min(resource max_height, processing max_dimension)
```

Validate dimensions before full pixel allocation where the container permits header-only inspection.

### 3.3 Verify every `ResourceLimits` field has a production site

Create a closure table in `plans/026-status.md` containing one row per field:

```text
limit field | production enforcement function | test name | observed failure type
```

Every current field must have at least one production enforcement site and one test:

- Input bytes.
- Width.
- Height.
- PNG chunk count.
- PNG chunk bytes.
- JPEG segment count.
- JPEG segment bytes.
- WebP RIFF chunk count.
- WebP RIFF chunk bytes.
- XMP bytes.
- XML depth.
- XML property count.
- Metadata field count.
- Metadata field bytes.
- Payload bytes.
- Detached-manifest bytes.
- Tile extraction origins.
- Verification seeds.

Tests must invoke externally reachable APIs rather than only calling limit helper methods directly.

### 3.4 Make `ResourceUsage` observed and honest

Do not populate a comprehensive-looking report with only input and output sizes.

Use a small operation-local accumulator passed through parser/processing boundaries, for example:

```rust
struct OperationBudget<'a> {
    limits: &'a ResourceLimits,
    usage: ResourceUsage,
}
```

Record actual observed values when work occurs:

- Container elements scanned.
- XMP bytes parsed.
- XML properties visited.
- Metadata fields and bytes copied.
- Payload bytes extracted or written.
- Tile origins checked.
- Verification seeds tried.
- Manifest, key, and signature records visited where represented by the public usage model.

If a metric cannot be measured reliably, do not fabricate it. Mark it unavailable or document it as an estimate. Do not label a simple output-buffer size as exact peak memory.

### 3.5 Fail-before-work tests

Add boundary and malicious-input tests proving:

- Metadata-only processing rejects oversized input before copying it.
- Default dimension limits apply without `max_dimension`.
- Oversized declared PNG/JPEG/WebP chunks fail before allocation/traversal.
- Excessive XML depth/property count fails before full extraction.
- Payload and manifest limits apply through public verification APIs.
- Tile-origin and verification-seed limits bound iteration.
- Resource failures are structured and stable in JSON/CLI classification.

### Gate 3 acceptance criteria

- Metadata-only processing cannot bypass input or dimension limits.
- Every `ResourceLimits` field appears in the closure table with production code and a passing test.
- Public request, legacy, verification, and detached entrypoints use explicit or default limits consistently.
- `ExecutionReport.resource_usage` contains observed operation data, not placeholder zeros for work that occurred.
- No field is documented as enforced when it is not enforced.

---

## Gate 4: Align CI, documentation, and independent conformance evidence

### 4.1 Make library and CLI checks explicitly blocking

Use explicit workspace/package commands so the CLI integration suite cannot be omitted accidentally.

Recommended blocking commands:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast
cargo test -p stegoeggo-cli --all-features --no-fail-fast
cargo test --doc --workspace --exclude stegoeggo-fuzz
cargo package --workspace
```

If `cargo test --workspace --exclude stegoeggo-fuzz` already includes the CLI tests, retaining the explicit CLI command is acceptable as defense in depth.

Keep fuzzing on nightly through `cargo fuzz`; do not force the fuzz package into an unsupported stable test path.

Apply the same blocking test contract to:

- Main CI.
- Release-candidate validation.
- Publication preflight or the reusable validation workflow it consumes.
- `scripts/validate-release.sh`.

### 4.2 Correct the stable CLI contract

The current CLI uses positional image inputs and `--verify`; it does not expose default `protect` and `verify` subcommands.

For this narrow pass, update `STABILITY.md`, README examples, and migration documentation to describe the actual parser shape. Do not add new subcommands solely to make stale documentation true.

Document feature gates accurately:

- `keygen`: `signatures`.
- `sign`: `signatures` plus whatever manifest feature is actually required.
- `verify-manifest`: both `signatures` and `detached-manifest` if both are required by compilation and behavior.

Add CLI help snapshot or parser integration tests so documentation cannot drift silently.

### 4.3 Restore non-zero independent conformance requirements

Do not close source-aware evidence by leaving these minima at zero:

- `external_legacy_min`.
- `external_conflict_min`.
- `external_preservation_min`.

For this closure pass, require at least:

```text
external_legacy_min       >= 1
external_conflict_min     >= 1
external_preservation_min >= 1
```

Retain the existing requirements of at least one external canonical fixture per supported format and at least one alternate-prefix fixture.

Add independent fixtures whose authoring path does not call StegoEggo production writers.

#### Difficult-area example: ExifTool-authored conflicting fixture

Use a checked-in ExifTool configuration defining the required custom XMP properties, then generate a fixture with contradictory canonical and legacy values. Record:

- Exact ExifTool version.
- Exact command.
- Checked-in configuration path.
- Source image provenance.
- License.
- SHA-256 digest.

Conceptual command:

```bash
exiftool -config tests/fixtures/tools/ExifTool_config \
  -XMP-plus:DataMining=Prohibited \
  -XMP-Iptc4x3mpExt:DMI-AITraining=Allowed \
  -o conflicting-external.png base.png
```

Use the actual tag names supported by the checked-in config. Do not claim ExifTool authored a fixture if a custom byte injector authored it.

For external preservation evidence, create an independently authored image containing unrelated title/creator/license metadata, process it through StegoEggo, and prove those fields remain.

### 4.4 Validate provenance and reproducibility

Strict manifest validation must require for externally authored fixtures:

- Tool name.
- Tool version.
- Reproducible command or explicit historical exception.
- Existing checked-in generator/config path where referenced.
- License/provenance statement.
- Digest match.

A generated StegoEggo fixture must not count as independent simply because it imitates another tool.

### 4.5 Current-head workflow evidence

A local test transcript is insufficient for closure. Obtain and inspect:

- Green main CI for the exact closure SHA.
- Green fuzz smoke for the exact closure SHA.
- Green release-candidate run for the exact release SHA.
- Conformance report artifact.
- External-tool version artifact.
- Package inventory artifact.
- Semver report artifact.
- Commit/SHA artifact proving RC identity.

Record run URLs/IDs, conclusions, artifact names, and inspected key values in `plans/026-status.md`.

### Gate 4 acceptance criteria

- Main CI and RC explicitly test the CLI and library feature matrix.
- Fuzz remains a separate nightly/smoke contract and all 12 targets are enumerated.
- Stable CLI documentation matches `--help` and feature compilation.
- Independent legacy, conflict, and preservation minima are non-zero and satisfied by truthfully classified fixtures.
- Removing each independent category causes strict conformance failure.
- Green current-head CI, fuzz smoke, and RC evidence is recorded and artifact contents are inspected.

---

## Gate 5: Correct ledgers and publish a new immutable patch release

### 5.1 Create truthful plan status ledgers

Create or correct:

- `plans/021-status.md`
- `plans/022-status.md`
- `plans/023-status.md`
- `plans/024-status.md`
- `plans/025-status.md`
- `plans/026-status.md`

Each ledger must include:

- Plan disposition: complete, partially superseded, or remaining work.
- Implementation commit SHAs.
- Acceptance criterion mapping.
- Exact validation commands.
- Test counts and ignored-test explanation.
- CI and RC run IDs/URLs.
- Artifact names and inspected results.
- Known limitations and non-goals.
- Release version containing the change.

Do not mark an item complete based only on a commit message.

Correct stale claims in `plans/024-status.md`, including:

- Fuzz target count.
- Resource-limit depth.
- CLI exit behavior.
- Current workflow commands.
- The distinction between the already-published `0.2.2` artifact and later fixes on `main`.

### 5.2 Prepare a new patch version

Because corrected code landed after `v0.2.2`, choose a new patch version, normally `0.2.3` unless another version has already been reserved or published.

Update consistently:

- Root crate version.
- CLI crate version and dependency requirement.
- Fuzz dependency requirement.
- `Cargo.lock`.
- Changelog.
- Documentation/version strings that are intended to name the current release.
- Conformance fixture authoring version only when fixtures are intentionally regenerated.

Do not regenerate conformance fixtures merely to change a version string unless their bytes are expected to change and the change is reviewed.

### 5.3 Changelog and security disclosure

The release notes must clearly state:

- Real Ed25519 replaced the earlier development construction.
- Payload-v3 extraction/capacity fixes.
- Detached-manifest CLI verification convergence.
- Resource-boundary fixes.
- JPEG parser panic regression fixed after `0.2.2`.
- Whether users of `0.2.2` should upgrade immediately.

Do not imply that signature validity proves ownership or legal authority.

### 5.4 Release rehearsal

For the exact release commit:

1. Run the blocking validation script from a clean checkout.
2. Run the release-candidate workflow.
3. Inspect every uploaded artifact.
4. Verify package contents by unpacking the `.crate` archives.
5. Install the CLI from the packaged crate or a temporary local registry.
6. Execute smoke tests for:
   - PNG/JPEG/WebP protect and verify.
   - V3 CRC and HMAC verification.
   - Key generation, sign, and detached verification with features enabled.
   - Untrusted, wrong-image, wrong-payload-key, and malformed-manifest exit codes.
7. Confirm the publish workflow will use the same validated SHA.

### 5.5 Publication and post-publication verification

Only after `plans/026-status.md` is complete through RC evidence:

- Create the new immutable tag.
- Publish the library first.
- Wait for index availability.
- Publish the CLI against the published library version.
- Create the GitHub release from the validated SHA.
- Verify checksums and binary `--version`.
- Verify crates.io versions and dependency resolution.
- Install the CLI from crates.io and repeat a minimal protect/verify smoke test.
- Record all publication evidence in `plans/026-status.md`.

### Gate 5 acceptance criteria

- The `v0.2.2` tag and published crates are unchanged.
- Current fixed code has a new version.
- Plans 021–026 have truthful status ledgers.
- The release SHA equals the successful RC SHA.
- Package contents, CLI installation, and smoke tests pass from published artifacts.
- Publication evidence is recorded with exact versions, SHAs, run IDs, and artifact checksums.

---

## Required validation matrix

The implementation agent must run and record at least:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast
cargo test -p stegoeggo-cli --all-features --no-fail-fast
cargo test --doc --workspace --exclude stegoeggo-fuzz
cargo test --test external_tools -- --ignored
cargo package --workspace
cargo deny check licenses
cargo deny check advisories
cargo audit
```

Feature-specific verification must include:

```bash
cargo test -p stegoeggo --no-default-features
cargo test -p stegoeggo --no-default-features --features signatures
cargo test -p stegoeggo --no-default-features --features detached-manifest
cargo test -p stegoeggo --no-default-features --features signatures,detached-manifest
cargo test -p stegoeggo --all-features
cargo test -p stegoeggo-cli --all-features
```

Run all 12 fuzz targets through the CI smoke workflow. Locally, at minimum run the changed targets long enough to cover their seed corpora and include any regression corpus added during this pass.

Run strict conformance with the same command used by CI and release-candidate validation.

## Review checklist

Before declaring completion, a reviewer must answer yes to all of the following:

- Does a v3 extraction path read its declared exact length rather than a v2-sized window?
- Does malformed v3 fail without legacy reinterpretation?
- Are CRC and HMAC channel flags truthful?
- Can a capacity skip ever produce `stego_succeeded = true`?
- Does detached CLI parsing use the bounded parser?
- Does detached CLI verification use the library verifier?
- Can manifest trust metadata affect caller trust?
- Is embedded-reference extraction raw-byte based?
- Are missing and wrong HMAC keys distinguishable?
- Are input and dimension limits applied before metadata-only work?
- Does every resource-limit field have a production site and public-boundary test?
- Does CI explicitly execute CLI tests?
- Do stable CLI docs match actual `--help` output?
- Are external legacy/conflict/preservation minima non-zero?
- Are current-head CI, fuzz, RC, and artifact records present?
- Is corrected code released under a version newer than `0.2.2`?

## Definition of done

This plan is complete only when:

1. Gates 1–4 are implemented and pass locally.
2. All required tests and feature combinations pass.
3. Strict conformance passes with non-zero independent category minima.
4. Main CI and fuzz smoke are green for the exact release commit.
5. The RC workflow is green for that same commit and its artifacts have been inspected.
6. Plans 021–026 status ledgers are committed and truthful.
7. A new patch release is published from the validated SHA.
8. Post-publication installation and smoke verification pass.
9. `plans/026-status.md` records all evidence and contains no unresolved blocking item.

If any item above is missing, leave Plan 026 open and do not describe the line of work as closed.