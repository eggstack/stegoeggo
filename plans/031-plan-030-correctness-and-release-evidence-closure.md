# Plan 031: Plan 030 Correctness and Release-Evidence Closure

Status: Ready for implementation

Baseline: `main` at `158015dc84b9f0bae58ebf6af77179b57dbb2ed9`

Depends on:

- `plans/029-plan-028-corrective-closure.md`
- `plans/030-plan-029-residual-corrective-handoff.md`
- `plans/029-status.md`
- `plans/030-status.md`

Release target: `0.3.0`

Release hold: keep `0.3.0` unpublished and untagged until every blocking criterion in this plan is satisfied against one exact code candidate SHA. A green development-branch CI run is necessary but is not release closure.

## Purpose

Plan 030 produced useful improvements, but its `CLOSED` disposition exceeds the implementation and evidence currently present on `main`. This plan is a narrowly scoped corrective handoff for the remaining correctness and release-evidence work.

This plan is written for a smaller implementation model. It therefore provides:

- exact production areas to inspect;
- required control-flow semantics;
- implementation examples;
- adversarial test matrices;
- focused commands after each phase;
- narrow commit boundaries;
- explicit evidence requirements;
- acceptance criteria that cannot be completed by changing comments, changelogs, or status files alone.

The current repository is build-healthy. Do not treat this as a broad rewrite. Preserve the working API, detached-verification improvements, existing v1/v2 compatibility, and green CI while correcting the remaining overclaims.

---

# Verified residual defects at the baseline

The implementation agent must begin from these facts rather than from the `CLOSED` claims in `plans/030-status.md`.

1. Non-tiled LSB and DCT extraction read six v3 prefix bytes, but they do not perform a separate declared-header extraction and complete header validation before full payload extraction.
2. Tiled LSB extraction and verification still begin with `V3_PROBE_BITS = 384` and extract at least 48 bytes.
3. Any tiled DCT/F5 path using the same fixed probe or fixed payload candidate behavior must be converted as well.
4. `classify_v3_probe` validates only basic magic, version, length, maximum embedded size, and a limited tag-length relationship.
5. V3 classification is not parameterized by the operation's `ResourceLimits`.
6. Authentication algorithm, exact tag length, extension layout, critical extension semantics, key-ID layout, reserved fields, and all checked arithmetic are not validated before full extraction.
7. `process_request_bytes_with_warnings` returns resolution warnings but does not derive runtime warnings from the observed `EmbedOutcomeSummary`.
8. Payload channel and path flags are still constructed from `ProtectionContext` request state, including a hard-coded `hidden_marker: true`.
9. `OperationBudget` is created after processing in the report path and is populated with synthetic one-item, zero-byte observations.
10. Parser, metadata, extraction, seed, tile-origin, payload, and allocation work is not observed through the budget where that work occurs.
11. Detached caller-key semantics are substantially improved at library level, but the full adversarial matrix is not tested through the actual CLI process, human output, JSON output, and exit codes.
12. Independent fixture provenance still claims or embeds ExifTool information even when Python manually writes the XMP.
13. Some fixtures were reclassified as `external` to satisfy a coverage minimum rather than being proven genuinely external.
14. Negative conformance coverage is incomplete.
15. Main CI and the local validation script cover more than the release-candidate workflow.
16. The release-candidate workflow does not invoke the authoritative validation contract and has no recorded exact-SHA successful run for the candidate.
17. The status ledger claims CI, fuzz, RC, package, and smoke closure without an RC run ID, packaged-crate installation evidence, or packaged CLI security smoke evidence.
18. `plans/030-status.md` is marked `CLOSED` while publication and post-publication criteria remain pending.

---

# Non-goals

Do not add or redesign:

- payload v4;
- a new image format;
- a new steganography algorithm;
- a new signature algorithm;
- certificate-chain validation;
- a built-in trust store;
- C2PA support;
- network services;
- broad performance refactors;
- unrelated CLI commands;
- unrelated metadata schemas;
- a new public rights-policy model;
- unrelated crate restructuring.

Do not rewrite functioning v1/v2 extraction.

Do not remove caller-owned detached key binding.

Do not reduce existing resource-limit defaults merely to make tests pass.

Do not relabel an internally generated fixture as external to satisfy a minimum.

Do not move, recreate, or modify any published `0.2.2` tag or artifact.

---

# Mandatory small-model execution rules

1. Work phases in order.
2. Use the suggested commit boundaries.
3. Keep production changes and evidence-only changes in separate commits.
4. Before editing a phase, list the exact functions and tests that will change in `plans/031-status.md`.
5. Run the focused test commands before moving to the next phase.
6. A helper is not completion until every governed production path calls it.
7. A resource-limit test is not completion unless it proves bounded failure at the public production entrypoint.
8. Do not call a fixed 48-byte read a prefix or header probe.
9. Only a definitive `NotV3` result permits v2/v1 fallback.
10. Do not infer runtime success by re-verifying output.
11. Do not reconstruct resource work after the operation.
12. Do not make CI checks non-blocking.
13. Do not add broad semver, audit, advisory, or license exemptions.
14. Do not update the changelog before behavior and tests are complete.
15. Do not mark a phase `CLOSED` while a required matrix row is missing.
16. Status values are only `OPEN`, `PARTIAL`, `CLOSED`, or `SUPERSEDED`.
17. Use full 40-character SHAs in release evidence.
18. A later documentation commit must not be confused with the exact code candidate SHA.
19. If a command fails, record the exact failing command and output summary before changing code.
20. Preserve the current green baseline while making narrow corrections.

Every phase requires all of the following before it may be marked `CLOSED`:

- production implementation;
- focused tests;
- public-path or subprocess test where applicable;
- exact validation commands and results;
- implementation commit SHA;
- truthful ledger row.

---

# Phase 0: Reset the status truth and establish a closure ledger

## 0.1 Correct the existing overclaim

Change `plans/030-status.md` from `Disposition: CLOSED` to `Disposition: PARTIAL`.

Do not erase the existing successful CI and fuzz evidence. Add a corrective section that states which Plan 030 criteria remain open and points to Plan 031.

At minimum, mark these Plan 030 definition-of-done rows as open or partial:

- every v3 path uses prefix, declared-header validation, and exact extraction;
- payload claims and capacity reflect actual emitted evidence;
- `EmbedOutcome` reaches warnings, human output, and strict exits;
- every resource limit is enforced and observed through production paths;
- complete CLI adversarial matrix;
- truthful independent provenance and complete negative conformance coverage;
- CI/RC contract alignment;
- one exact candidate passes CI, fuzz, RC, package, and smoke;
- publication and post-publication verification.

## 0.2 Create `plans/031-status.md`

Initialize it with:

```text
Plan baseline SHA: 158015dc84b9f0bae58ebf6af77179b57dbb2ed9
Code candidate SHA: not selected
Evidence commit SHA: not selected
Release version: 0.3.0
Disposition: OPEN
Release hold: active
```

Add the following required tables.

### Table A: v3 extraction inventory

Columns:

```text
carrier/path | extraction function | verification function | prefix exact | header exact | payload exact | limits applied | no legacy fallback test | status
```

Required rows:

- non-tiled PNG/WebP LSB;
- tiled PNG/WebP LSB;
- non-tiled JPEG DCT/F5;
- tiled JPEG DCT/F5 if implemented;
- raw-byte known-seed path;
- metadata-seed wrapper;
- fixed-seed fallback wrapper;
- detached embedded-reference verification.

### Table B: runtime outcome propagation

Columns:

```text
entrypoint | plan warnings | runtime warnings | embed summary | metadata summary | human output | JSON output | strict exit | status
```

Required public entrypoints:

- `process_request_bytes`;
- `process_request_bytes_with_warnings`;
- `process_request_bytes_with_report`;
- legacy byte-processing wrappers used by the CLI;
- CLI single-file path;
- CLI batch path.

### Table C: resource limits

One row for every `ResourceLimits` getter or builder field in the current source.

Columns:

```text
limit | enforcement function | production callers | usage observation | bounded-failure public test | error variant | status
```

Do not manually omit fields. Generate or verify the inventory by searching all `max_*` accessors and builder methods.

### Table D: detached CLI matrix

Columns:

```text
case | expected overall status | expected exit | human assertion | JSON assertion | subprocess test | status
```

### Table E: conformance provenance

Columns:

```text
fixture | source class | actual writer | exact writer version | generator SHA | generation command | digest | negative coverage | status
```

### Table F: release evidence

Columns:

```text
code candidate SHA | main CI run | RC run | fuzz run | package artifacts | smoke run | tag | publication | post-publication | status
```

## Phase 0 acceptance criteria

- `plans/030-status.md` no longer claims complete closure.
- `plans/031-status.md` exists with all six required tables.
- Every currently open item is represented by a row.
- No technical work is marked complete without a code SHA and command result.
- `0.3.0` remains unpublished and untagged.

## Suggested commit

```text
plans: correct Plan 030 disposition and open Plan 031 ledger
```

---

# Phase 1: Complete a shared three-stage v3 extraction contract

This phase is blocking and must be completed before outcome, resource, or release closure is claimed.

## 1.1 Remove the fixed probe from production extraction

Remove production dependence on:

```rust
const V3_PROBE_BITS: usize = 384;
```

The constant may be deleted. Do not keep it as the initial tiled probe.

Search for and remove all production patterns equivalent to:

```rust
extract_lsb(..., V3_PROBE_BITS, ...)
max(total_bits, V3_PROBE_BITS)
for bits in [V3_CRC_PAYLOAD_BITS, V3_HMAC_PAYLOAD_BITS, ...]
```

Legacy fixed-size constants may remain only inside v1/v2 fallback code reached after a definitive `NotV3` result.

## 1.2 Define one prefix result and one validated-header result

Use the existing `payload_v3` wire-format implementation as the source of truth. Do not create a second parser with different validation rules.

Recommended internal shapes:

```rust
const V3_PREFIX_BYTES: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum V3PrefixResult {
    NotV3,
    Detected {
        header_length: usize,
        total_length: usize,
    },
    UnsupportedVersion(u8),
    Malformed(PayloadMalformedReason),
    ResourceLimitExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidatedV3Header {
    pub header_length: usize,
    pub total_length: usize,
    pub auth_algorithm: AuthAlgorithm,
    pub auth_tag_length: usize,
    pub key_id_length: usize,
    pub extension_count: usize,
    pub flags: PayloadFlags,
    pub channels: ProtectionChannels,
}
```

Exact names may differ. Required separation:

1. prefix classification reads only the first six bytes;
2. header validation reads exactly the declared header;
3. full payload validation reads exactly the declared total length.

## 1.3 Prefix validation

Implement a function equivalent to:

```rust
fn classify_v3_prefix(
    prefix: &[u8],
    limits: &ResourceLimits,
) -> V3PrefixResult;
```

Required checks:

- exactly six bytes are available before reading fields;
- first two magic bytes match;
- version byte is supported;
- `header_length >= V3_CORE_SIZE`;
- `total_length >= header_length`;
- `total_length <= limits.max_payload_bytes()`;
- `total_length <= V3_MAX_EMBEDDED_SIZE` when the format has a hard wire maximum;
- byte-to-bit conversion uses `checked_mul(8)`;
- no allocation or full extraction occurs before these checks.

Required semantics:

- wrong first two magic bytes: `NotV3`;
- correct magic plus unsupported version: `UnsupportedVersion`;
- correct magic plus malformed lengths: `Malformed`;
- correct magic plus resource excess: structured resource failure;
- short carrier that cannot provide six bytes: extraction-level insufficient capacity or not found, not legacy decoding.

## 1.4 Declared-header validation

Split or reuse the current v3 parser so a header can be validated without requiring the authentication tag or full payload.

Validate all header fields before full payload extraction:

- header byte count equals `header_length`;
- core fields fit inside the header;
- authentication algorithm is supported;
- authentication tag length exactly matches the selected algorithm;
- key-ID length fits inside the declared header;
- extension count is bounded;
- every TLV header fits;
- every TLV value fits;
- cumulative offsets use checked arithmetic;
- critical unknown extension fails;
- noncritical unknown extension is skipped safely;
- reserved bits and reserved bytes obey the wire contract;
- channel and flag encodings reject impossible or reserved values when required by the format;
- `header_length + auth_tag_length == total_length`, unless the existing wire specification explicitly permits additional authenticated body bytes;
- `total_length` remains within both operation and wire limits.

If the existing full parser already performs these checks, refactor it into shared header and full-payload stages. Do not copy and diverge the logic.

## 1.5 Shared extraction driver

Use a closure, small internal trait, or carrier adapter so every stego carrier follows the same state machine.

Illustrative shape:

```rust
fn extract_v3_candidate<F>(
    mut extract_exact_bits: F,
    available_bits: usize,
    limits: &ResourceLimits,
    mac_key: &[u8],
    trace: Option<&mut ExtractionTrace>,
) -> CandidateOutcome
where
    F: FnMut(usize) -> Option<Vec<u8>>,
{
    let prefix_bits = V3_PREFIX_BYTES.checked_mul(8)
        .ok_or(CandidateOutcome::MalformedV3)?;

    let prefix = match extract_exact_bits(prefix_bits) {
        Some(bytes) => bytes,
        None => return CandidateOutcome::NotFound,
    };

    let lengths = match classify_v3_prefix(&prefix, limits) {
        V3PrefixResult::NotV3 => return CandidateOutcome::NotV3,
        V3PrefixResult::Detected { header_length, total_length } => {
            (header_length, total_length)
        }
        V3PrefixResult::UnsupportedVersion(v) => {
            return CandidateOutcome::UnsupportedVersion(v)
        }
        V3PrefixResult::Malformed(_) => return CandidateOutcome::MalformedV3,
        V3PrefixResult::ResourceLimitExceeded => {
            return CandidateOutcome::ResourceLimitExceeded
        }
    };

    let header_bits = match lengths.0.checked_mul(8) {
        Some(bits) => bits,
        None => return CandidateOutcome::MalformedV3,
    };
    let header = match extract_exact_bits(header_bits) {
        Some(bytes) => bytes,
        None => return CandidateOutcome::InsufficientCapacity,
    };
    let validated = match validate_v3_header(&header, limits) {
        Ok(value) => value,
        Err(error) => return map_v3_header_error(error),
    };

    let total_bits = match validated.total_length.checked_mul(8) {
        Some(bits) => bits,
        None => return CandidateOutcome::MalformedV3,
    };
    if total_bits > available_bits {
        return CandidateOutcome::InsufficientCapacity;
    }

    let full = match extract_exact_bits(total_bits) {
        Some(bytes) => bytes,
        None => return CandidateOutcome::InsufficientCapacity,
    };

    verify_v3_payload_exact(&full, &validated, mac_key)
}
```

This is pseudocode. Adapt error handling to current repository types. Do not copy invalid `?` usage into an enum-returning function.

The state-machine requirement is mandatory:

```text
six-byte prefix -> exact declared header -> exact declared payload -> integrity/authentication
```

## 1.6 Fallback semantics

Only `NotV3` may enter legacy v2/v1 extraction.

These outcomes must terminate the candidate path without legacy decoding:

- v3 magic with unsupported version;
- malformed header length;
- malformed total length;
- payload over resource limit;
- header capacity failure;
- payload capacity failure;
- unsupported authentication algorithm;
- invalid authentication tag length;
- malformed extension;
- critical unknown extension;
- missing HMAC key;
- wrong HMAC key;
- CRC failure;
- any v3 parser failure after magic classification.

When multiple seed or tile candidates are tried, another independent candidate may be tried if the current candidate does not classify as v3. Once a candidate provides v3 magic, do not reinterpret those same extracted bits as legacy ECC.

## 1.7 Convert every path

Update the inventory in `plans/031-status.md` and convert all applicable functions in:

- `src/protected/steganography.rs`;
- v3 parser modules under `src/payload_v3/`;
- raw embedded-reference helpers;
- detached embedded-reference verification wrappers.

Required carrier/path coverage:

1. Non-tiled LSB extraction.
2. Non-tiled LSB verification.
3. Tiled LSB extraction.
4. Tiled LSB verification.
5. Non-tiled DCT/F5 extraction.
6. Non-tiled DCT/F5 verification.
7. Tiled DCT/F5 extraction, if present.
8. Tiled DCT/F5 verification, if present.
9. Known-seed wrappers.
10. Metadata-seed wrappers.
11. Fixed-seed fallback wrappers.
12. Raw extraction used for detached references.
13. Detached embedded-reference verification.

Do not mark the phase complete with only non-tiled LSB and DCT converted.

## 1.8 Test-only control-flow instrumentation

Add a test-only trace or callback.

Example:

```rust
#[cfg(test)]
#[derive(Default, Debug)]
struct ExtractionTrace {
    requested_bit_lengths: Vec<usize>,
    prefix_extractions: usize,
    header_extractions: usize,
    full_extractions: usize,
    legacy_decoder_entries: usize,
}
```

Tests must be able to prove:

- first v3 request is exactly 48 bits;
- second request is exactly `header_length * 8`;
- third request is exactly `total_length * 8`;
- a 36-byte CRC payload is not read as 48 bytes;
- malformed or unsupported v3 never increments `legacy_decoder_entries`.

Do not expose this instrumentation in the public API.

## 1.9 Required tests

Add focused tests with names equivalent to:

```text
v3_non_tiled_lsb_uses_prefix_header_exact_payload
v3_tiled_lsb_uses_prefix_header_exact_payload
v3_non_tiled_dct_uses_prefix_header_exact_payload
v3_tiled_dct_uses_prefix_header_exact_payload
v3_crc_payload_does_not_request_48_bytes
v3_hmac_payload_requests_declared_length
v3_variable_header_with_key_id_round_trips
v3_variable_header_with_noncritical_extension_round_trips
v3_unknown_critical_extension_fails_without_legacy
v3_malformed_extension_length_fails_without_legacy
v3_unsupported_auth_algorithm_fails_without_legacy
v3_wrong_auth_tag_length_fails_without_legacy
v3_declared_total_over_resource_limit_fails_before_full_extract
v3_total_length_bit_overflow_is_rejected
v3_header_length_greater_than_total_is_rejected
v3_insufficient_header_capacity_is_not_legacy
v3_insufficient_payload_capacity_is_not_legacy
v3_missing_hmac_key_is_not_legacy
v3_wrong_hmac_key_is_not_legacy
v3_crc_failure_is_not_legacy
v3_shaped_legacy_ecc_collision_does_not_decode_as_legacy
legacy_v1_still_extracts
legacy_v2_still_extracts
```

The variable-header tests must produce payloads longer than the old 36-byte and 48-byte assumptions.

## Phase 1 focused commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p stegoeggo --all-features v3_ -- --nocapture
cargo test -p stegoeggo --all-features tiled -- --nocapture
cargo test -p stegoeggo --all-features dct -- --nocapture
cargo test -p stegoeggo --all-features legacy -- --nocapture
cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast
```

## Phase 1 acceptance criteria

- No production extraction path uses `V3_PROBE_BITS` or an equivalent fixed 48-byte initial probe.
- Every v3 carrier uses the same three-stage state machine.
- Exact declared header validation occurs before full extraction.
- Full parser validation is shared rather than duplicated.
- Operation `max_payload_bytes` is checked before full extraction and allocation.
- Only `NotV3` enters legacy decoding.
- All required path rows are `CLOSED` in Table A.
- Test instrumentation proves exact requested lengths and zero legacy entries for v3 failures.
- v1 and v2 compatibility tests remain green.

## Suggested commits

```text
stego: split v3 prefix and declared-header validation
stego: convert tiled and raw paths to exact v3 extraction
stego: add no-fallback and variable-header adversarial tests
```

---

# Phase 2: Make payload claims, warnings, reports, CLI output, and strict exits truthful

## 2.1 Introduce one operation result

Replace separate post-hoc inference paths with one internal result returned by plan execution.

Recommended shape:

```rust
pub(crate) struct ProcessingOutcome {
    pub bytes: Vec<u8>,
    pub metadata: MetadataOutcomeSummary,
    pub embed: Option<EmbedOutcomeSummary>,
    pub runtime_warnings: Vec<ProtectionWarning>,
    pub resource_usage: ResourceUsage,
}
```

`PipelineResult` may be expanded or replaced. The exact name is not important.

Required properties:

- output bytes and observed channel results travel together;
- metadata success is recorded where metadata is written;
- embed success, skip, degradation, path, payload bytes, required capacity, and available capacity come from `EmbedOutcome`;
- runtime warnings are derived from observed outcomes;
- resource usage is supplied by the operation-local budget from Phase 3;
- public wrappers project from this one result rather than rerunning or reparsing work.

## 2.2 Define metadata outcome explicitly

Recommended shape:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MetadataStatus {
    NotRequested,
    Injected,
    PreservedExisting,
    ReplacedStegoOwned,
    SkippedUnsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MetadataOutcomeSummary {
    pub status: MetadataStatus,
    pub fields_written: usize,
    pub bytes_written: usize,
    pub output_format: ImageOutputFormat,
}
```

Do not determine metadata success by scanning the finished output as the primary signal. A verification scan may remain as a test assertion, not production truth.

## 2.3 Build payloads from a resolved emission context

Do not generate payload flags from a generic mutable `ProtectionContext` with hard-coded channel values.

Introduce an internal resolved embed context after output format and path selection.

Example:

```rust
pub(crate) struct PayloadEmissionContext {
    pub rights_metadata_planned: bool,
    pub embed_path: EmbedPath,
    pub tiled: bool,
    pub progressive_output: bool,
    pub authentication: AuthenticationMode,
    pub key_id: Option<Vec<u8>>,
    pub extensions: Vec<PayloadExtension>,
}
```

Required semantics:

- `hidden_marker` inside a payload is true because the payload is being generated for an actual embed attempt, not because a generic context hard-codes it;
- `tiled` reflects the selected embed path;
- `progressive_jpeg` reflects the actual selected output mode and must not claim unsupported DCT embedding after fallback;
- authentication reflects the actual serialized algorithm;
- rights-metadata claim reflects the resolved plan;
- key-ID and extension flags derive from actual serialized fields;
- payload is serialized before capacity calculation;
- every capacity calculation uses `payload.len()` from the exact serialized payload.

For a capacity skip, no payload was emitted. The report must state `SkippedCapacity`; do not claim hidden-marker success because bytes were generated in memory.

## 2.4 Derive runtime warnings from observed outcomes

Add a single mapping function.

Example:

```rust
fn warnings_from_outcome(
    plan: &ResolvedProtectionPlan,
    outcome: &ProcessingOutcome,
) -> Vec<ProtectionWarning>;
```

Required mappings include:

- `EmbedStatus::SkippedCapacity` plus LSB path -> `LsbCapacitySkipped`;
- `EmbedStatus::SkippedCapacity` plus DCT path -> `DctCapacityInsufficient`;
- progressive unsupported or fallback -> `ProgressiveJpegFallback`;
- required hidden marker not embedded -> error-severity warning or structured operation error according to the existing request contract;
- best-effort hidden marker not embedded -> warning, not false success;
- metadata requested but not written -> metadata degradation warning or error according to the existing request contract.

Deduplicate warnings by variant and relevant detail. Preserve resolution warnings and append runtime warnings.

## 2.5 Make all public entrypoints project from the same outcome

Required behavior:

### `process_request_bytes`

- executes once;
- returns `outcome.bytes`;
- returns an error when a required channel failed according to request semantics.

### `process_request_bytes_with_warnings`

- executes once;
- returns resolution plus runtime warnings;
- reports observed capacity skips and progressive fallback.

### `process_request_bytes_with_report`

- executes once;
- uses observed metadata and embed summaries;
- uses Phase 3 resource usage;
- does not rescan output to infer primary success;
- uses the same warning vector as the warning API.

### CLI

- human and JSON output display the same observed status;
- strict exit selection uses the report/outcome, not preflight guesses;
- batch aggregation preserves per-file observed results.

## 2.6 Required tests

Add tests equivalent to:

```text
warnings_api_reports_observed_lsb_capacity_skip
warnings_api_reports_observed_dct_capacity_skip
report_and_warnings_api_return_same_runtime_warnings
report_uses_embed_outcome_without_reverification
metadata_only_report_has_no_embed_summary
metadata_only_report_records_observed_metadata_injection
best_effort_capacity_skip_returns_output_and_warning
required_marker_capacity_skip_is_failure
crc_payload_capacity_uses_serialized_length
hmac_payload_capacity_uses_serialized_length
variable_key_id_capacity_uses_serialized_length
extension_payload_capacity_uses_serialized_length
tiled_report_path_matches_actual_embed_path
progressive_fallback_payload_does_not_claim_dct_embedding
progressive_fallback_report_is_degraded
cli_human_capacity_skip_matches_json
cli_strict_exit_fails_on_required_marker_skip
cli_non_strict_best_effort_skip_succeeds_with_warning
batch_report_preserves_each_file_embed_outcome
```

Tests must assert exact `payload_bytes`, `required_capacity`, and `available_capacity` where deterministic.

## Phase 2 focused commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p stegoeggo --test request_api --all-features -- --nocapture
cargo test -p stegoeggo-cli --all-features -- --nocapture
cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast
```

## Phase 2 acceptance criteria

- One operation result carries bytes, metadata result, embed result, warnings, and usage.
- `process_request_bytes_with_warnings` includes observed runtime warnings.
- Reports and warning APIs agree for the same operation.
- CLI human, JSON, and strict exits use the same observed outcome.
- Payload flags derive from resolved serialized fields, not hard-coded generic context state.
- Capacity uses the actual serialized payload length in every path.
- Required and best-effort channel failures have distinct behavior.
- Table B is complete and every row is `CLOSED`.

## Suggested commits

```text
pipeline: introduce canonical processing outcome
payload: derive flags and capacity from resolved emission context
cli: drive warnings and strict exits from observed outcomes
```

---

# Phase 3: Thread resource enforcement and observation through actual work

## 3.1 Do not create the budget after processing

Remove the current report-path pattern that creates `OperationBudget` after `process_plan_bytes` and records a synthetic zero-byte container item.

The budget must be created before any governed work and passed through that work.

Required start order for public byte entrypoints:

```text
create budget -> check input size -> inspect bounded header -> check dimensions -> parse/scan/decode/embed/write -> finish budget
```

## 3.2 Strengthen the budget API

Recommended shape:

```rust
pub(crate) struct OperationBudget<'a> {
    limits: &'a ResourceLimits,
    usage: ResourceUsage,
    largest_buffer_bytes: usize,
}

impl OperationBudget<'_> {
    pub fn check_input(&mut self, bytes: usize) -> Result<()>;
    pub fn record_png_chunk(&mut self, bytes: usize) -> Result<()>;
    pub fn record_jpeg_segment(&mut self, bytes: usize) -> Result<()>;
    pub fn record_webp_chunk(&mut self, bytes: usize) -> Result<()>;
    pub fn record_xmp_bytes(&mut self, bytes: usize) -> Result<()>;
    pub fn record_metadata_field(&mut self, bytes: usize) -> Result<()>;
    pub fn record_payload_probe(&mut self, bytes: usize) -> Result<()>;
    pub fn record_payload_extract(&mut self, bytes: usize) -> Result<()>;
    pub fn record_tile_origin(&mut self) -> Result<()>;
    pub fn record_verification_seed(&mut self) -> Result<()>;
    pub fn record_buffer(&mut self, bytes: usize);
    pub fn finish(self, output_bytes: usize) -> ResourceUsage;
}
```

Each `record_*` method must both enforce the relevant limit and update usage where a limit exists.

Do not silently saturate a count and continue.

## 3.3 Correct allocation terminology

`peak_allocations_bytes` is not a true process peak-memory measurement unless allocator instrumentation exists.

Choose one:

1. Rename it before `0.3.0` to `largest_buffer_bytes` and document exactly what it measures.
2. Keep the public field for compatibility but deprecate it and populate a new truthful `largest_buffer_bytes` field.
3. Introduce real allocator-backed peak measurement, only if it remains narrow and portable.

Preferred option: truthful largest-buffer tracking.

Record buffer sizes at actual allocations or capacity reservations, not by comparing only input and output lengths after processing.

## 3.4 Thread the budget through production modules

At minimum, pass `&mut OperationBudget` or a narrow observer interface through:

- request resolution execution boundary;
- PNG chunk scanning and rewriting;
- JPEG header/segment parsing and DCT traversal;
- WebP RIFF scanning and rewriting;
- XMP extraction and merge;
- metadata field extraction and copy;
- v3 prefix/header/full-payload extraction;
- seed fallback loops;
- tile-origin loops;
- detached embedded-reference extraction;
- detached manifest verification where image work is performed;
- conformance parsing when public resource limits govern it.

Avoid a global mutable budget.

Avoid thread-local hidden state.

## 3.5 Enforce limits before useful work exceeds the cap

Examples:

### Verification seed limit

A cap of one must prevent attempting the second seed.

Test design:

- construct an image whose valid marker exists only under the second configured test seed;
- verify with `max_verification_seeds(1)` and assert bounded failure/not found;
- verify with `max_verification_seeds(2)` and assert success;
- assert usage is one and two respectively.

### Tile-origin limit

- construct a cropped/tiled image whose valid tile is found only at origin two;
- cap one must fail;
- cap two must succeed;
- assert usage exactly matches attempts.

### Payload limit

- construct a valid v3 prefix declaring a payload larger than the cap;
- assert failure occurs after six-byte prefix and before header/full extraction;
- trace must show no full-payload request.

### Chunk and segment limits

- create valid files with work beyond the cap;
- public processing must fail before processing the first over-limit item;
- usage must report only allowed items attempted.

## 3.6 Public entrypoint inventory

Inventory every public function accepting bytes or image data, including legacy wrappers.

For each entrypoint state one of:

- directly creates a budget;
- receives a budget from its canonical caller;
- explicitly documented as a low-level unbounded primitive not exposed to untrusted input.

No public untrusted-byte entrypoint may be omitted from Table C.

## 3.7 Required tests

Provide a public bounded-failure test for every current resource limit.

At minimum cover:

```text
max_input_bytes
max_width
max_height
max_png_chunks
max_png_chunk_bytes
max_jpeg_segments
max_jpeg_segment_bytes
max_webp_riff_chunks
max_webp_chunk_bytes if present
max_xmp_bytes
max_metadata_fields
max_metadata_field_bytes
max_payload_bytes
max_tile_extraction_origins
max_verification_seeds
any manifest-record or extension limits present in ResourceLimits
```

Add usage accuracy tests equivalent to:

```text
resource_usage_counts_actual_png_chunks
resource_usage_counts_actual_jpeg_segments
resource_usage_counts_actual_webp_chunks
resource_usage_records_xmp_bytes
resource_usage_records_metadata_fields_and_bytes
resource_usage_records_v3_prefix_header_and_payload
resource_usage_records_exact_seed_attempts
resource_usage_records_exact_tile_origins
resource_usage_largest_buffer_is_observed_not_posthoc
report_resource_usage_matches_parser_trace
```

## Phase 3 focused commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p stegoeggo --all-features resource -- --nocapture
cargo test -p stegoeggo --test request_api --all-features resource -- --nocapture
cargo test -p stegoeggo --test detached_manifest_tests --all-features resource -- --nocapture
cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast
```

## Phase 3 acceptance criteria

- No report path constructs usage after processing.
- No synthetic `observe_*_chunk(0)` reporting remains.
- Every `ResourceLimits` field has an enforcement and observation row.
- Every limit has a public bounded-failure test.
- Seed and tile-origin tests prove useful work exists beyond the cap.
- V3 payload limits stop work before full extraction.
- Allocation reporting uses truthful terminology and actual observations.
- Public byte entrypoints are inventoried and bounded.
- Table C is complete and every row is `CLOSED`.

## Suggested commits

```text
limits: make operation budget enforce and observe actual work
limits: thread budget through metadata and container parsers
limits: thread budget through stego and detached verification
limits: add public bounded-failure and usage-accuracy matrix
```

---

# Phase 4: Complete detached verification through the real CLI surface

Library tests are necessary but not sufficient. This phase must execute the compiled CLI as a subprocess.

## 4.1 Preserve the existing library semantics

Retain:

- early malformed-manifest return;
- caller-owned key bytes used for cryptographic verification;
- manifest key bytes compared with caller-owned bytes;
- key-material mismatch priority before signature failure;
- exit code `3` for integrity/configuration contradictions;
- exit code `4` only for valid but untrusted evidence;
- key-ID-only trust explicitly documented as legacy behavior.

## 4.2 Parse and validate the manifest before image work in the CLI

The CLI should:

1. read bounded manifest bytes;
2. parse the manifest;
3. validate manifest structure;
4. parse and validate caller key input;
5. only then read/hash/decode the image and verify signatures.

A malformed or duplicate-key manifest must exit `2` before image hashing or embedded extraction.

Use test-only hooks or an unreadable/very large image path to prove invalid manifest rejection occurs first.

## 4.3 Define stable trust-mode output

Human and JSON output must identify one of:

```text
none
key-id-only
caller-verifying-key
callback
```

The CLI normally uses `none` or `caller-verifying-key`. Do not describe caller-owned key verification as key-ID-only trust.

For every signature expose:

- key ID;
- structural validity;
- cryptographic validity;
- key-ID match;
- key-material match;
- trusted result;
- source.

Do not expose private key bytes or raw secret material.

## 4.4 Add a dedicated subprocess test file

Preferred path:

```text
stegoeggo-cli/tests/verify_manifest_cli.rs
```

Use temporary directories and deterministic test keys generated through library helpers or the CLI `keygen` command.

Create actual manifest JSON files. For duplicate/conflicting records, write raw JSON directly rather than using builder methods that deduplicate.

## 4.5 Required CLI adversarial matrix

### Case A: correct caller key

```text
signature key ID A
manifest key ID A
manifest key bytes A
signature made by A
caller key A
```

Expected:

- overall `VerifiedTrusted`;
- exit `0`;
- cryptographic valid true;
- key ID match true;
- key material match true;
- trusted true;
- trust mode `caller-verifying-key`.

### Case B: substituted manifest bytes under trusted ID

```text
signature key ID A
manifest key ID A
manifest key bytes B
signature made by A
caller key A
```

Expected:

- `KeyMaterialMismatch`;
- exit `3`;
- cryptographic valid true using caller A;
- key ID match true;
- key material match false;
- trusted false.

### Case C: attacker self-consistent substitution

```text
signature key ID A
manifest key ID A
manifest key bytes B
signature made by B
caller key A
```

Expected:

- `KeyMaterialMismatch`, not ordinary untrusted;
- exit `3`;
- key material match false;
- trusted false.

### Case D: correct manifest bytes, wrong signature

```text
signature key ID A
manifest key ID A
manifest key bytes A
signature made by B
caller key A
```

Expected:

- `SignatureFailure`;
- exit `3`;
- key material match true;
- cryptographic valid false.

### Case E: no caller key

Valid manifest and signature, no trust anchor.

Expected:

- `VerifiedUntrusted`;
- exit `4`;
- cryptographic valid true;
- trusted false;
- trust mode `none`.

### Case F: unrelated caller key

Valid A manifest/signature, caller supplies key C under ID C.

Expected:

- valid but untrusted;
- exit `4`;
- no material mismatch attributed to A;
- trusted false.

### Case G: malformed caller key

Expected:

- exit `2`;
- no signature verification;
- stable human and JSON configuration diagnostic.

### Case H: duplicate manifest key IDs

Expected:

- exit `2` before image verification;
- no signature rows claiming cryptographic evaluation.

### Case I: duplicate signature records when prohibited

Expected behavior must follow manifest validation rules and be tested explicitly.

### Case J: wrong image digest

Expected:

- `BindingFailure`;
- exit `3`;
- human and JSON agree.

### Case K: embedded HMAC reference without key

Expected:

- `AuthenticationKeyMissing` embedded-reference status;
- exit `3` when the embedded reference is required by the manifest.

### Case L: embedded HMAC reference with wrong key

Expected:

- `AuthenticationFailed`;
- exit `3`.

## 4.6 Human and JSON agreement

For every matrix case, execute both normal output and JSON output.

Assert:

- same overall status;
- same exit code;
- same trust mode;
- same per-signature booleans;
- no private key bytes;
- no contradictory labels such as `trusted` in one mode and `untrusted` in another.

## Phase 4 focused commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p stegoeggo --test detached_manifest_tests --all-features -- --nocapture
cargo test -p stegoeggo-cli --test verify_manifest_cli --all-features -- --nocapture
cargo test -p stegoeggo-cli --all-features --no-fail-fast
cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast
```

## Phase 4 acceptance criteria

- All twelve CLI matrix cases execute the compiled binary.
- Human and JSON assertions exist for each security-relevant case.
- Exit `4` is used only for valid but untrusted evidence.
- Key-material contradictions always exit `3`.
- Malformed manifests and caller keys exit `2` before expensive image work.
- Trust mode is explicit and accurate.
- Table D is complete and every row is `CLOSED`.

## Suggested commits

```text
cli: validate detached inputs before image verification
cli: add caller-key trust mode and stable diagnostics
cli: add detached adversarial subprocess matrix
```

---

# Phase 5: Correct independent conformance provenance and negative coverage

## 5.1 Define source classes accurately

Use these meanings:

- `generated`: produced by StegoEggo production code or repository test helpers;
- `independent`: produced by a repository generator that does not call production writers;
- `external`: imported or generated by a named third-party writer actually invoked to write the metadata;
- `legacy`: historical fixture retained from an earlier release, with recorded provenance.

If the manifest schema currently accepts only `generated` and `external`, extend it before `0.3.0` or document a precise mapping. Do not call a Python manual chunk injector `external` merely because it is not production Rust.

## 5.2 Correct the independent generator

For `tests/fixtures/conformance/independent/generate_independent_fixtures.sh`:

- record exact Python version;
- record exact ImageMagick version;
- record the generator file digest or repository commit SHA;
- remove `any recent version`;
- do not claim ExifTool unless the script actually invokes ExifTool to write the fixture;
- remove or replace `x:xmptk="ExifTool 12.76"` when Python wrote the XMP;
- if `xmptk` is retained, identify the actual independent generator;
- make regeneration deterministic where possible;
- regenerate fixture digests through a single command;
- record the exact command in the manifest.

A truthful manual-writer record may look like:

```text
authoring_tool = "python-stdlib-png-xmp-injector"
authoring_tool_version = "Python 3.13.5"
generator_revision = "<full git SHA>"
generation_command = "tests/fixtures/conformance/independent/generate_independent_fixtures.sh"
```

Do not assign the ExifTool version to a Python writer.

## 5.3 Add genuinely external fixtures when external minima are required

If the strict harness requires external alternate-prefix coverage, create or import fixtures using a real third-party metadata writer.

Examples:

- ExifTool invoked by command line to write XMP;
- exiv2 invoked by command line;
- libvips or ImageMagick only if that tool actually writes the required metadata field.

Record:

- tool name;
- exact version;
- exact command;
- base fixture digest;
- resulting fixture digest;
- license and source;
- generator revision.

Do not relabel `canonical_alt_prefix_png` or similar internal fixtures as external unless provenance proves an external writer authored them.

## 5.4 Add provenance validation to the harness

Strict conformance must fail when:

- an external fixture has no exact tool version;
- generation command is missing;
- generator revision is missing where repository-generated;
- digest is wrong;
- claimed authoring tool conflicts with embedded `xmptk` or recorded process metadata;
- an internal fixture is mislabeled external;
- required external categories or formats are absent;
- alternate-prefix external coverage falls below the configured minimum.

Do not rely only on comments in `manifest.toml`.

## 5.5 Complete the conflict truth table

Test all four states through the strict harness:

```text
expected false, observed false -> pass
expected true, observed true   -> pass
expected false, observed true  -> fail
expected true, observed false  -> fail
```

The two failing states must assert the correct diagnostic direction.

Inspect all canonical and legacy DMI values found in the fixture, not only the first value.

## 5.6 Add negative coverage tests

Create test copies of the manifest or temporary fixture directories and prove strict conformance fails for:

- removed external PNG category;
- removed external JPEG category;
- removed alternate-prefix fixture;
- source reclassified from external to generated;
- source reclassified from generated to external without provenance;
- incorrect digest;
- missing license;
- missing generation command;
- missing exact version;
- missing generator revision;
- contradictory authoring-tool metadata;
- expected conflict reversed;
- required external field missing;
- minimum coverage reduced below the policy floor.

Do not change production fixture minima downward to satisfy a test.

## 5.7 Real preservation proof

Use a genuinely independent fixture containing unrelated XMP and metadata not owned by StegoEggo.

Through the public request API:

1. read and record the unrelated fields before processing;
2. apply rights metadata;
3. read and record the unrelated fields after processing;
4. assert byte or semantic preservation as appropriate;
5. assert StegoEggo-owned fields were updated correctly;
6. execute for each supported container where preservation is promised.

At minimum cover PNG, JPEG, and WebP when the public preservation contract applies to all three.

## Phase 5 focused commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p stegoeggo --test preservation --all-features -- --nocapture
cargo test -p stegoeggo --all-features conformance -- --nocapture
./scripts/verify_metadata_conformance.sh
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict \
  --json conformance-report.json
```

## Phase 5 acceptance criteria

- No fixture claims ExifTool authorship unless ExifTool actually wrote it.
- Exact writer versions and generator revisions are recorded.
- Internal fixtures are not relabeled external to satisfy minima.
- Required external coverage is provided by genuinely external fixtures.
- Strict provenance validation rejects inaccurate records.
- All four conflict states are tested.
- Complete negative coverage tests pass by observing intended harness failures.
- Public preservation tests retain unrelated fields.
- Table E is complete and every row is `CLOSED`.

## Suggested commits

```text
conformance: correct independent fixture provenance
conformance: add genuine external alternate-prefix fixtures
conformance: enforce provenance and negative coverage
conformance: prove unrelated metadata preservation through public API
```

---

# Phase 6: Make CI and release-candidate validation one authoritative contract

## 6.1 Refactor the validation script into explicit phases

Keep one authoritative implementation under `scripts/validate-release.sh`.

Recommended interface:

```bash
scripts/validate-release.sh --phase hermetic
scripts/validate-release.sh --phase external
scripts/validate-release.sh --phase package-smoke
scripts/validate-release.sh --phase all --expected-sha <40-char-sha>
```

Alternative sub-scripts are acceptable only when `validate-release.sh` invokes them and workflows do not duplicate command lists independently.

### Hermetic phase must include

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast
cargo test -p stegoeggo-cli --all-features --no-fail-fast
cargo test --doc --workspace --exclude stegoeggo-fuzz
cargo package --workspace
cargo deny check licenses
cargo deny check advisories
cargo audit
cargo semver-checks check-release
cargo +1.87 check --workspace --all-features
scripts/check_fuzz_sync.sh
```

### Feature phase must include at least

```bash
cargo test -p stegoeggo --no-default-features
cargo test -p stegoeggo --no-default-features --features async
cargo test -p stegoeggo --no-default-features --features signatures
cargo test -p stegoeggo --no-default-features --features detached-manifest
cargo test -p stegoeggo --no-default-features --features signatures,detached-manifest
cargo test -p stegoeggo --all-features
cargo test -p stegoeggo-cli --all-features
```

Adjust combinations to the actual feature graph. Record unsupported combinations explicitly rather than silently omitting them.

### External phase must include

```bash
cargo test --test external_tools -- --ignored
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance --strict ...
```

## 6.2 Main CI must call the authoritative contract

Main CI may retain parallel jobs, but each job must invoke an authoritative script phase rather than maintain an independent command list.

Example:

```yaml
- run: scripts/validate-release.sh --phase hermetic
```

If parallelization requires narrower phases, those phases must be implemented in the script and called identically by RC.

## 6.3 RC must require an exact SHA

Change the RC workflow input:

```yaml
inputs:
  sha:
    description: Full 40-character commit SHA
    required: true
```

After checkout:

```bash
actual="$(git rev-parse HEAD)"
test "$actual" = "${{ inputs.sha }}"
test "${#actual}" -eq 40
```

Fail on branches, abbreviated SHAs, moving tags, or checkout mismatch.

## 6.4 RC must invoke the full authoritative contract

The RC workflow must run:

```bash
scripts/validate-release.sh --phase all --expected-sha "$EXPECTED_SHA"
```

Do not retain a narrower root-only clippy/test/doc command list.

RC must therefore include:

- workspace clippy;
- workspace tests;
- explicit CLI tests;
- workspace docs;
- feature matrix;
- package checks;
- deny;
- audit;
- semver;
- MSRV;
- fuzz synchronization;
- external integration;
- strict conformance;
- package smoke from packaged sources.

## 6.5 Add exact-candidate fuzz workflow

The existing normal CI does not need to run all fuzz targets on every push. Add or correct a manually dispatched release-fuzz workflow that:

- requires a full SHA;
- verifies checkout equality;
- discovers the target list from `fuzz/Cargo.toml`;
- verifies workflow/script synchronization;
- runs every target for the documented duration;
- uses the same code candidate SHA;
- uploads logs and crash artifacts even on failure;
- records toolchain and cargo-fuzz versions;
- fails if any target is missing or crashes.

Minimum release-candidate duration: 60 seconds per target unless a longer repository policy already exists.

## 6.6 Required RC artifacts

Upload:

- `commit-sha.txt`;
- `commit-info.txt`;
- full validation log;
- semver report;
- audit report;
- deny report;
- conformance JSON report;
- external tool versions;
- package inventories for every workspace publishable crate;
- packaged `.crate` archives;
- SHA-256 checksums for package archives;
- package-smoke log;
- CLI security-smoke log.

Artifacts must include the exact candidate SHA in metadata or filenames.

## Phase 6 focused checks

```bash
scripts/validate-release.sh --phase hermetic
scripts/validate-release.sh --phase external
scripts/check_fuzz_sync.sh
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml")'
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/release-candidate.yml")'
```

Use an available YAML parser if Ruby is not supported in the project environment.

## Phase 6 acceptance criteria

- Main CI and RC call the same authoritative script phases.
- RC no longer has a narrower root-only contract.
- RC requires and verifies a full SHA.
- Audit, feature tests, CLI tests, fuzz sync, package smoke, and external conformance are all blocking in RC.
- Release fuzz discovers and runs every target against the exact SHA.
- Required artifacts are uploaded.
- No release-critical `continue-on-error` remains.

## Suggested commits

```text
ci: make release validation script authoritative
ci: align main and RC validation phases
ci: add exact-SHA release fuzz and evidence artifacts
```

---

# Phase 7: Produce exact-SHA package and security-smoke evidence

This phase begins only after Phases 1-6 are closed.

## 7.1 Freeze one code candidate

Select a full 40-character code candidate SHA `C`.

Requirements:

- clean tree;
- all technical changes committed;
- release version consistently `0.3.0`;
- no pending source, test, fixture, workflow, or script changes;
- status ledger may still be `PARTIAL` because external runs are pending.

Record `C` in `plans/031-status.md` only in a later evidence commit. Do not amend or rebuild `C` after runs begin.

## 7.2 Required runs against the same SHA

Run and record:

1. Main CI run against `C`.
2. Release-candidate workflow against input `C`.
3. Release-fuzz workflow against input `C`.

All must succeed.

If any run exposes a defect:

- fix the defect in a new commit;
- select a new candidate SHA;
- rerun all three workflows;
- do not combine evidence from different candidates.

## 7.3 Package smoke must use packaged source

Do not run smoke tests only from the repository checkout.

For each publishable crate:

1. obtain the `.crate` archive generated by RC;
2. verify its checksum;
3. unpack it into a clean temporary directory;
4. inspect the file inventory;
5. build/test from unpacked package contents.

### Library consumer smoke

Create a clean temporary Rust crate with a path dependency pointing at the unpacked `stegoeggo-0.3.0` package.

The consumer must compile and run tests covering:

- metadata-only PNG operation;
- hidden marker PNG operation;
- JPEG operation;
- WebP operation;
- execution report inspection;
- v3 verification;
- detached caller-key verification when features are enabled.

### CLI package smoke

Install from the unpacked packaged CLI source with `--locked`.

Run:

- `--help`;
- PNG protect and verify;
- JPEG protect and verify;
- WebP protect and verify;
- metadata-only request;
- capacity-skip warning case;
- JSON report case;
- strict required-marker failure case;
- detached trusted verification;
- detached untrusted exit `4`;
- detached key-material mismatch exit `3`;
- malformed manifest exit `2`.

Use fresh temporary directories and no repository-local target artifacts.

## 7.4 Evidence commit versus code candidate

After all runs succeed, create an evidence-only commit `E` updating status ledgers.

`E` may be later than candidate `C`, but it must not change release code.

Before using `C` for publication, verify:

```bash
git diff --name-only C..E
```

Only evidence/status documentation files may differ.

If source, tests, fixtures, workflows, scripts, manifests, Cargo files, or package contents differ, select a new code candidate and rerun all gates.

Record both:

```text
Code candidate SHA: C
Evidence commit SHA: E
```

Do not tag `E` as the release unless the full validation suite is rerun against `E`.

## 7.5 Correct all historical ledgers

Update Plans 021-031 truthfully.

Each applicable status file must include:

- disposition;
- implementation SHA or superseding plan;
- exact commands;
- run IDs;
- artifact names;
- unresolved limitations;
- release relation.

Do not rewrite history to imply an earlier plan completed work implemented only by Plan 031.

## Phase 7 acceptance criteria

- One full SHA passed main CI, RC, and all fuzz targets.
- All three recorded run IDs resolve to that SHA.
- RC artifacts were downloaded and inspected.
- Package smoke used unpacked package sources.
- CLI security smokes used the packaged CLI.
- Evidence from multiple candidate SHAs was not combined.
- The evidence commit differs from the code candidate only in documentation/evidence files.
- Table F contains run IDs, artifact names, and results.
- Plan 031 remains `PARTIAL` until publication and post-publication verification are complete.

## Suggested commits

```text
plans: record exact-SHA RC fuzz and package-smoke evidence
plans: reconcile Plans 021-031 against final candidate
```

---

# Phase 8: Publish and perform post-publication verification

This phase is release execution, not implementation. Do not begin it automatically merely because development CI is green.

## 8.1 Pre-publication checks

From a clean worktree checked out at code candidate `C`:

```bash
test "$(git rev-parse HEAD)" = "C"
test -z "$(git status --porcelain)"
scripts/validate-release.sh --phase all --expected-sha C
```

Confirm:

- `0.3.0` is not already published for either crate;
- Cargo package versions and dependency constraints are consistent;
- changelog heading is correct;
- README and documentation do not claim unsupported behavior;
- package checksums match RC artifacts;
- release notes identify breaking changes from `0.2.2`.

## 8.2 Publication sequence

Use the exact code candidate `C`.

Recommended order:

1. create an immutable annotated tag for `v0.3.0` at `C`;
2. publish the library crate;
3. wait only for registry availability required by the CLI dependency;
4. publish the CLI crate;
5. create the GitHub release from tag `v0.3.0`;
6. attach checksums, package inventories, conformance report, and release evidence.

Do not rebuild from a different ref.

Do not force-move the tag.

## 8.3 Post-publication clean-environment verification

In a clean environment with no repository path dependencies:

```bash
cargo install stegoeggo-cli --version 0.3.0 --locked
```

Create a fresh consumer using the registry version of `stegoeggo = "0.3.0"` and run the same library smoke cases used for the packaged source.

Run the installed CLI security matrix subset:

- trusted detached verification -> exit `0`;
- valid untrusted verification -> exit `4`;
- key-material mismatch -> exit `3`;
- malformed manifest -> exit `2`;
- wrong image binding -> exit `3`;
- missing embedded HMAC key -> exit `3`.

Verify published package contents, checksums, docs.rs build, and repository/tag links.

## 8.4 Final closure

Create a final evidence-only status commit after post-publication checks.

Mark Plan 031 `CLOSED` only when:

- both crates are published at `0.3.0`;
- tag points to `C`;
- GitHub release points to the tag;
- clean registry installs succeed;
- post-publication security smokes pass;
- all run IDs and checksums are recorded;
- no unresolved blocker remains.

Plans 026-030 may be marked `SUPERSEDED` or `CLOSED` only with a precise explanation of how Plan 031 completed their remaining criteria.

## Phase 8 acceptance criteria

- Publication used exactly `C`.
- Tag is immutable and resolves to `C`.
- Library and CLI registry versions are `0.3.0`.
- Clean registry consumer and CLI installation succeed.
- Post-publication security smokes have expected exit codes.
- Final status contains checksums and release URLs or identifiers.
- Plan 031 contains no unresolved blocker.

## Suggested commit

```text
plans: close 0.3.0 release with post-publication evidence
```

---

# Required implementation order

Use this order without skipping ahead:

1. Phase 0: correct status truth.
2. Phase 1: complete v3 extraction.
3. Phase 2: complete runtime outcome propagation.
4. Phase 3: thread resource enforcement and observation.
5. Phase 4: complete detached CLI adversarial verification.
6. Phase 5: correct conformance provenance and negative coverage.
7. Phase 6: align CI, RC, validation, and release fuzz.
8. Run the full local validation suite.
9. Select a code candidate SHA.
10. Phase 7: obtain exact-SHA CI, RC, fuzz, package, and smoke evidence.
11. Phase 8: publish only after explicit release execution and verify registry artifacts.

Do not run release publication while any technical phase is `OPEN` or `PARTIAL`.

---

# Suggested commit sequence

Keep commits narrow and reviewable.

```text
1.  plans: correct Plan 030 disposition and open Plan 031 ledger
2.  stego: split v3 prefix and declared-header validation
3.  stego: convert tiled and raw paths to exact v3 extraction
4.  stego: add no-fallback and variable-header adversarial tests
5.  pipeline: introduce canonical processing outcome
6.  payload: derive flags and capacity from resolved emission context
7.  cli: drive warnings and strict exits from observed outcomes
8.  limits: make operation budget enforce and observe actual work
9.  limits: thread budget through metadata and container parsers
10. limits: thread budget through stego and detached verification
11. limits: add public bounded-failure and usage-accuracy matrix
12. cli: validate detached inputs before image verification
13. cli: add caller-key trust mode and stable diagnostics
14. cli: add detached adversarial subprocess matrix
15. conformance: correct independent fixture provenance
16. conformance: add genuine external alternate-prefix fixtures
17. conformance: enforce provenance and negative coverage
18. conformance: prove unrelated metadata preservation through public API
19. ci: make release validation script authoritative
20. ci: align main and RC validation phases
21. ci: add exact-SHA release fuzz and evidence artifacts
22. plans: record exact-SHA RC fuzz and package-smoke evidence
23. plans: reconcile Plans 021-031 against final candidate
24. plans: close 0.3.0 release with post-publication evidence
```

If a commit needs an unrelated change to pass, explain the dependency in the commit message and status ledger. Do not silently fold unrelated cleanup into a phase.

---

# Mandatory local validation before candidate selection

Run from a clean tree:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast
cargo test -p stegoeggo-cli --all-features --no-fail-fast
cargo test --doc --workspace --exclude stegoeggo-fuzz
cargo package --workspace
cargo deny check licenses
cargo deny check advisories
cargo audit
cargo semver-checks check-release
cargo +1.87 check --workspace --all-features
cargo test -p stegoeggo --no-default-features
cargo test -p stegoeggo --no-default-features --features async
cargo test -p stegoeggo --no-default-features --features signatures
cargo test -p stegoeggo --no-default-features --features detached-manifest
cargo test -p stegoeggo --no-default-features --features signatures,detached-manifest
cargo test -p stegoeggo --all-features
cargo test -p stegoeggo-cli --all-features
cargo test --test external_tools -- --ignored
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict \
  --json conformance-report.json
scripts/check_fuzz_sync.sh
```

After Phase 6, the equivalent preferred command is:

```bash
scripts/validate-release.sh --phase all --expected-sha "$(git rev-parse HEAD)"
```

No command may be replaced by a comment stating it is expected to pass.

---

# Mandatory repository searches before closure

Record results in `plans/031-status.md`.

```bash
rg -n "V3_PROBE_BITS|max\(total_bits, V3_PROBE_BITS\)" src tests
rg -n "V3_CRC_PAYLOAD_BITS|V3_HMAC_PAYLOAD_BITS" src
rg -n "observe_(png|jpeg|webp)_chunk\(0\)" src
rg -n "CLOSED \(partial\)|Disposition: CLOSED" plans/*-status.md
rg -n "any recent version|ExifTool 12\.76|python\+imagemagick" tests/fixtures/conformance
rg -n "continue-on-error" .github/workflows
rg -n "cargo clippy --all-targets|cargo test --all-features|cargo test --doc$" .github/workflows/release-candidate.yml
```

Expected closure results:

- no fixed v3 production probe;
- fixed v3 payload constants only in explicitly justified test or legacy-independent capacity code, preferably none in capacity logic;
- no synthetic zero-byte usage observations;
- no `CLOSED (partial)` status;
- Plan 030 not falsely closed before Plan 031 completion;
- no inaccurate independent provenance strings;
- no release-critical `continue-on-error`;
- RC invokes the authoritative script rather than a narrower command list.

---

# Reviewer checklist

A reviewer must answer every item with code and test evidence.

## V3 extraction

- Does every path begin with exactly six extracted bytes?
- Is the exact declared header extracted next?
- Is the exact declared total payload extracted only after full header validation?
- Does `ResourceLimits::max_payload_bytes` apply before full extraction?
- Are authentication and extension fields validated by shared parser code?
- Can any v3-magic failure enter legacy ECC?
- Do tiled and non-tiled paths share semantics?
- Do DCT and LSB paths share semantics?
- Do detached raw-reference paths share semantics?

## Runtime outcomes

- Is there one canonical operation result?
- Are warnings derived from observed outcomes?
- Does the warnings API include capacity and progressive runtime degradation?
- Do report and warnings APIs agree?
- Does CLI human output agree with JSON?
- Do strict exits use observed required-channel success?
- Does capacity use the exact serialized payload length?

## Resources

- Is the budget created before governed work?
- Is it passed through real parsers and extractors?
- Does each limit have a public bounded-failure test?
- Do seed and tile tests prove the cap changes success?
- Is allocation terminology truthful?
- Does reported usage match actual traces?

## Detached CLI

- Are malformed manifests rejected before image work?
- Are caller key bytes used for crypto?
- Are manifest key bytes compared against caller bytes?
- Is attacker substitution exit `3`?
- Is valid untrusted evidence exit `4`?
- Do human and JSON results agree?
- Are all required cases subprocess tests?

## Conformance

- Did the named writer actually write each fixture?
- Are exact versions and revisions recorded?
- Are external fixtures genuinely external?
- Does strict mode reject false provenance?
- Are all four conflict states tested?
- Are negative coverage tests complete?
- Are unrelated fields preserved through public operations?

## Release validation

- Do CI and RC invoke the same script contract?
- Does RC require a full SHA and verify checkout?
- Are audit, feature, CLI, package, conformance, and smoke checks blocking?
- Does release fuzz run every discovered target against the same SHA?
- Are artifacts sufficient to reproduce the decision?
- Did packaged-source smokes pass?
- Are candidate and evidence SHAs distinguished?

---

# Definition of done

Plan 031 is complete only when all of the following are true.

1. `plans/030-status.md` accurately reflects its partial/superseded state.
2. `plans/031-status.md` contains complete inventories and evidence tables.
3. Every v3 carrier uses six-byte prefix, exact declared header, and exact declared payload extraction.
4. No production path uses a fixed 48-byte v3 probe.
5. V3 authentication, extensions, key IDs, lengths, reserved fields, and limits are validated before full extraction.
6. No v3-magic failure can enter legacy decoding.
7. V1 and v2 compatibility remains green.
8. One canonical operation result carries bytes, metadata, embed outcome, warnings, and resource usage.
9. Warning APIs, reports, human output, JSON output, and strict exits agree with observed outcomes.
10. Capacity calculations use exact serialized payload lengths.
11. Payload flags derive from the resolved actual embed plan rather than hard-coded generic context state.
12. `OperationBudget` is created before work and threaded through actual production operations.
13. Every current resource limit has production enforcement, observed usage, and a public bounded-failure test.
14. Resource usage contains no synthetic zero-byte reconstruction.
15. Allocation reporting is truthfully named and observed.
16. The complete detached adversarial matrix passes through the compiled CLI.
17. Key-material contradictions exit `3`; valid untrusted evidence exits `4`; malformed configuration exits `2`.
18. Independent and external fixture provenance is accurate and reproducible.
19. Strict conformance rejects provenance, digest, source-class, conflict, and coverage failures.
20. Public preservation tests prove unrelated metadata survives.
21. Main CI and RC execute one authoritative validation contract.
22. RC requires and verifies a full candidate SHA.
23. Release fuzz runs every target against the same candidate SHA with zero crashes.
24. One full code candidate SHA passes main CI, RC, fuzz, package, and packaged-source smoke tests.
25. Evidence from different candidate SHAs is not combined.
26. Candidate and evidence commits are recorded separately.
27. `0.3.0` is published only from the exact validated code candidate.
28. The immutable tag resolves to that candidate.
29. Clean registry library and CLI installations succeed.
30. Post-publication detached security smokes return the documented exit codes.
31. All Plans 021-031 have truthful dispositions and evidence.
32. No unresolved blocker remains in `plans/031-status.md`.

Until criteria 1-26 are complete, the release disposition is `NO-GO`.

Until criteria 27-32 are complete, Plan 031 remains `PARTIAL` and may not be marked `CLOSED`.
