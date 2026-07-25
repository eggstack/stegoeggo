# Plan 030: Plan 029 Residual Corrective Handoff

Status: Ready for implementation

Baseline: `main` at `1ad9cc192460ce0efc6ce91ce25674b5f421d9c6`

Depends on:

- `plans/028-final-trust-payload-and-release-closure.md`
- `plans/029-plan-028-corrective-closure.md`
- `plans/029-status.md`

Release hold: keep `0.2.3` unreleased. Do not tag, publish, create a GitHub release, or claim Plans 026-030 are closed until every blocking criterion in this plan is satisfied against one exact candidate SHA.

## Purpose

This plan addresses only the defects that remained after the attempted Plan 029 implementation. It is written as a deterministic handoff for a smaller implementation model. Follow the phases in order, keep commits narrow, and do not substitute comments, changelog entries, or status-ledger assertions for production implementation and public-path tests.

Current blocking defects:

1. Non-tiled LSB and DCT extraction still begin with fixed 36-byte and 48-byte v3 payload candidates.
2. The tiled probe still extracts a fixed 48-byte window rather than a minimum prefix followed by the declared header and exact payload length.
3. V3 classification does not validate the complete header, extensions, authentication fields, resource limits, or checked arithmetic before full extraction.
4. Several paths perform integrity or legacy ECC work before v3 classification.
5. Payload channel claims and capacity decisions are still based on requested configuration and fixed constants rather than actual serialized and emitted evidence.
6. `EmbedOutcome` is discarded before warning, report, JSON, human, and strict-exit decisions.
7. Resource enforcement and usage accounting are reconstructed after processing rather than enforced and observed where work occurs.
8. Detached manifest validation still continues into hashing and signature evaluation after structural validation fails.
9. Caller-key diagnostics remain incomplete at the CLI, and true CLI attacker-substitution tests are absent.
10. Independent fixture provenance remains internally inconsistent; negative coverage and real preservation tests are absent.
11. Main CI makes semver non-blocking, while RC and local validation execute different command contracts.
12. Public API changes around `TrustPolicy` are not demonstrably patch-compatible with `0.2.2`.
13. Historical ledgers and exact-SHA CI, fuzz, RC, package, publication, and post-publication evidence are absent.

## Non-goals

Do not add:

- payload v4;
- a new image format;
- a new steganography algorithm;
- a new signature algorithm;
- certificate-chain validation;
- a built-in trust store;
- C2PA integration;
- broad performance refactors;
- unrelated CLI redesign;
- unrelated metadata namespaces;
- new publication targets.

Do not modify, recreate, or move the published `0.2.2` tag or artifacts.

---

# Small-model execution rules

These rules are mandatory.

1. Work one phase at a time.
2. Use the suggested commit boundaries. Do not combine unrelated phases.
3. Before editing, identify the exact production functions and tests affected.
4. After each commit, run the focused tests listed for that phase.
5. Do not mark a criterion closed because a helper exists. The helper must be called by every governed production path.
6. Do not mark a resource cap closed with a success-only test. A bounded-failure test must prove that useful work exists beyond the cap.
7. Do not label a workstream `CLOSED (partial)`. Use only `OPEN`, `PARTIAL`, or `CLOSED`.
8. Do not update the changelog until the relevant behavior is implemented and public-path tests pass.
9. Do not make CI checks non-blocking to obtain green runs.
10. Do not add broad semver exemptions. Fix the API or select the correct release version.
11. Do not publish from a dirty tree, a rebuilt ref, or a SHA different from the validated candidate.
12. When a listed command fails, record the exact failure before changing code.
13. Preserve v1 and v2 compatibility unless a test proves the existing fixture is invalid.
14. Treat `plans/029-status.md` as potentially inaccurate. Correct it rather than copying its closure claims.

Every phase is incomplete until all of the following exist:

- production code;
- focused tests;
- public-path or CLI test where applicable;
- exact command and result;
- status-ledger entry tied to an implementation SHA.

---

# Phase 0: Establish the baseline and release-version decision

## 0.1 Record the current state

Create `plans/030-status.md` immediately with:

```text
Plan baseline SHA: 1ad9cc192460ce0efc6ce91ce25674b5f421d9c6
Candidate SHA: not selected
Release version: undecided pending semver remediation
Disposition: OPEN
```

Record current failures without describing them as expected or harmless.

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast
cargo test -p stegoeggo-cli --all-features --no-fail-fast
cargo test --doc --workspace --exclude stegoeggo-fuzz
cargo semver-checks check-release
```

Record the exact semver diagnostics, including item names and rule identifiers.

## 0.2 Choose the release path using this decision tree

Preferred path: preserve `0.2.3` patch compatibility.

1. Determine which public APIs existed in published `0.2.2`.
2. Restore those APIs to source-compatible shapes.
3. Move new caller-key functionality behind additive functions or new additive types.
4. Re-run `cargo semver-checks check-release`.
5. Keep `0.2.3` only if the full semver check passes with no broad suppression.

Fallback path: use `0.3.0` only when a required security or correctness change cannot be expressed compatibly.

Do not select `0.3.0` merely to avoid refactoring. If `0.3.0` is required, update all workspace package versions, dependency constraints, changelog headings, plans, and release commands consistently.

## 0.3 Patch-compatible trust API design

`TrustPolicy` existed in `0.2.2`. Adding a variant or adding `#[non_exhaustive]` to that already-published enum can break downstream exhaustive matches.

Preferred compatible design:

- restore `TrustPolicy` to the exact published variant set and exhaustiveness;
- keep legacy verification functions accepting `&TrustPolicy` unchanged;
- introduce a new additive caller-key input type;
- introduce a new additive verification function used by the CLI.

Example shape:

```rust
#[derive(Debug, Clone)]
pub struct CallerVerifyingKey {
    pub key_id: Vec<u8>,
    pub key: VerifyingKey,
}

#[derive(Debug, Default)]
pub struct DetachedVerificationOptions<'a> {
    pub trust_policy: Option<&'a TrustPolicy>,
    pub caller_verifying_keys: &'a [CallerVerifyingKey],
    pub payload_mac_key: Option<&'a [u8]>,
    pub limits: Option<&'a ResourceLimits>,
}

pub fn verify_detached_manifest_with_options(
    image_bytes: &[u8],
    manifest: &DetachedManifest,
    options: DetachedVerificationOptions<'_>,
) -> ManifestVerification;
```

The exact names may differ. Required properties:

- old callers compile unchanged;
- the CLI passes caller-owned public-key bytes through the new API;
- caller-owned key bytes are used for cryptographic verification;
- manifest key bytes are compared against caller-owned bytes when a matching manifest entry exists;
- the old key-ID-only policy remains explicitly legacy behavior.

`DetachedOverallStatus` may remain additive only if semver confirms it did not exist in the published baseline. If it existed, do not add a new variant in a patch release. Represent key-material mismatch through an additive detail field and map it to exit 3 in the CLI.

## Phase 0 acceptance criteria

- `plans/030-status.md` records the exact baseline commands and semver failures.
- Main CI no longer has `continue-on-error` on semver before this phase is marked closed.
- A documented release-version decision exists.
- If targeting `0.2.3`, `cargo semver-checks check-release` passes without a broad exemption.
- Existing `0.2.2` API compile fixtures pass.
- The CLI can still use caller-owned verifying-key bytes without relying on a new `TrustPolicy` variant.

## Suggested commit

```text
api: restore patch-compatible detached trust surface
```

---

# Phase 1: Implement a real minimum-prefix, header-first v3 extraction contract

This phase replaces all fixed v3 payload-size candidate loops. Do not retain them in production v3 classification paths.

## 1.1 Use three extraction stages

The v3 wire format exposes the following fields in the first six bytes:

```text
offset 0..1: magic
offset 2: version
offset 3: header_length
offset 4..5: total_length
```

Use three stages:

1. Prefix stage: extract exactly 6 bytes.
2. Header stage: after valid v3 magic/version and bounded lengths, extract exactly `header_length` bytes.
3. Payload stage: after full header validation, extract exactly `total_length` bytes.

Do not use 36-byte or 48-byte payload constants in stages 1 or 2.

## 1.2 Define one shared classifier

Recommended internal types:

```rust
const V3_PREFIX_BYTES: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum V3Probe {
    NotV3,
    Prefix {
        header_length: usize,
        total_length: usize,
    },
    Header(V3ValidatedHeader),
    Malformed(PayloadMalformedReason),
    UnsupportedVersion(u8),
    InsufficientCapacity,
    ResourceLimitExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct V3ValidatedHeader {
    pub header_length: usize,
    pub total_length: usize,
    pub auth_algorithm: AuthAlgorithm,
    pub auth_tag_length: usize,
    pub key_id_length: usize,
    pub extension_count: usize,
}
```

Separate functions are acceptable:

```rust
fn classify_v3_prefix(prefix: &[u8], limits: &ResourceLimits) -> V3Probe;
fn validate_v3_header(header: &[u8], limits: &ResourceLimits) -> Result<V3ValidatedHeader, PayloadMalformedReason>;
```

Do not call a 48-byte extraction a prefix probe.

## 1.3 Validate before full extraction

Prefix validation must check:

- exactly enough bytes to read magic, version, header length, and total length;
- `SE` magic;
- version 3;
- `header_length >= V3_CORE_SIZE`;
- `total_length >= header_length`;
- `total_length <= limits.max_payload_bytes()`;
- checked conversion from bytes to bits;
- carrier capacity for the requested header and full payload lengths.

Header validation must check:

- header bytes length equals declared `header_length`;
- supported authentication algorithm;
- exact valid tag length for the selected algorithm;
- key-ID length fits inside the declared header;
- extension count and every TLV length fit inside the declared header;
- no cumulative-length overflow;
- critical unknown extension fails;
- noncritical unknown extension is skipped safely;
- header end plus auth tag equals `total_length` when required by the format;
- reserved fields obey the existing format contract.

Use checked arithmetic:

```rust
let total_bits = total_length
    .checked_mul(8)
    .ok_or(PayloadMalformedReason::LengthOverflow)?;

let required_carrier_bits = total_bits
    .checked_mul(redundancy)
    .ok_or(PayloadMalformedReason::LengthOverflow)?;
```

## 1.4 Introduce one shared extraction driver

Use a closure or trait so LSB and DCT differ only in bit access.

Example:

```rust
fn probe_then_extract_v3<F>(
    mut extract_exact_bits: F,
    available_bits: usize,
    limits: &ResourceLimits,
    mac_key: &[u8],
) -> CandidateOutcome
where
    F: FnMut(usize) -> Option<Vec<u8>>,
{
    let prefix = extract_exact_bits(V3_PREFIX_BYTES * 8)
        .ok_or(CandidateOutcome::NotFound)?;

    let prefix = match classify_v3_prefix(&prefix, limits) {
        V3Probe::NotV3 => return CandidateOutcome::NotV3,
        V3Probe::UnsupportedVersion(v) => return CandidateOutcome::UnsupportedVersion(v),
        V3Probe::Malformed(reason) => return CandidateOutcome::MalformedV3(reason),
        V3Probe::Prefix { header_length, total_length } => (header_length, total_length),
        other => return map_probe_failure(other),
    };

    let header = extract_exact_bits(prefix.0 * 8)
        .ok_or(CandidateOutcome::InsufficientCapacity)?;
    let validated = validate_v3_header(&header, limits)
        .map_err(CandidateOutcome::MalformedV3)?;

    let full = extract_exact_bits(validated.total_length * 8)
        .ok_or(CandidateOutcome::InsufficientCapacity)?;
    verify_v3_payload(full, &validated, mac_key)
}
```

This is illustrative pseudocode. Adapt it to the repository's result conventions; do not copy invalid `?` use into an `enum`-returning function.

Required semantic contract:

- `NotV3` is the only result that permits v2/v1 fallback.
- Any v3 magic with malformed header, unsupported version, insufficient capacity, resource failure, missing key, wrong key, or invalid tag ends that candidate path without legacy decoding.

## 1.5 Convert every production path

Convert and inventory:

- non-tiled LSB extraction;
- non-tiled LSB verification;
- tiled LSB extraction;
- tiled LSB verification;
- non-tiled DCT/F5 extraction;
- non-tiled DCT/F5 verification;
- tiled DCT/F5 extraction;
- tiled DCT/F5 verification;
- raw-byte embedded-reference extraction;
- raw-byte embedded-reference verification;
- known-seed wrappers;
- metadata-seed wrappers;
- detached-manifest embedded-reference verification.

After conversion, production code must not contain initial loops such as:

```rust
for bits in [V3_CRC_PAYLOAD_BITS, V3_HMAC_PAYLOAD_BITS, ...]
```

Legacy constants may remain only in a function that is called after `V3Probe::NotV3`.

## 1.6 Add test-only instrumentation

Under `#[cfg(test)]`, add counters or a trace sink:

```rust
#[derive(Default)]
struct ExtractionTrace {
    prefix_extractions: usize,
    header_extractions: usize,
    full_extractions: usize,
    legacy_decoder_entries: usize,
}
```

Use it only to prove control flow. Do not expose it publicly.

## 1.7 Required tests

Add one focused test module for shared-probe behavior.

Required tests:

```text
v3_crc_with_extension_uses_declared_length_lsb
v3_hmac_with_key_id_and_extension_uses_declared_length_lsb
v3_crc_with_extension_uses_declared_length_dct
v3_tiled_lsb_classifies_before_legacy
v3_tiled_dct_classifies_before_legacy
v3_malformed_total_below_header_never_enters_legacy
v3_payload_above_resource_limit_fails_before_full_extract
v3_payload_above_carrier_capacity_fails_before_full_extract
v3_unsupported_version_never_enters_legacy
v3_unknown_auth_algorithm_is_malformed
v3_crc_wrong_tag_length_is_malformed
v3_hmac_wrong_tag_length_is_malformed
v3_key_id_overruns_header_is_malformed
v3_extension_overruns_header_is_malformed
v3_critical_unknown_extension_fails
v3_checked_length_overflow_fails
v3_missing_hmac_key_is_structured
v3_wrong_hmac_key_is_structured
v1_fixture_decodes_only_after_not_v3
v2_fixture_decodes_only_after_not_v3
```

The malformed-v3 tests must assert:

```rust
assert_eq!(trace.legacy_decoder_entries, 0);
```

## Phase 1 acceptance criteria

- Initial v3 classification extracts six bytes, not 36 or 48 bytes.
- Every v3 path uses the same prefix and header validators.
- Full extraction uses exactly `total_length * 8` bits.
- No v3-magic failure can enter legacy ECC or v1/v2 parsing.
- `max_payload_bytes` is enforced before header/full extraction as applicable.
- Header length, auth fields, key ID, extensions, critical extensions, capacity, and arithmetic are validated.
- All listed LSB, DCT, tiled, raw-byte, seed, and detached paths are mapped in `plans/030-status.md`.
- V1 and v2 fixtures remain green.

## Suggested commits

```text
stego: add bounded v3 prefix and header validators
stego: route all carriers through shared exact-length extraction
stego: add no-legacy-fallback adversarial coverage
```

---

# Phase 2: Propagate actual embedding and metadata outcomes

## 2.1 Add explicit emission inputs

Do not serialize claims directly from generic request configuration.

Recommended shape:

```rust
#[derive(Debug, Clone)]
pub(crate) struct PayloadEmissionContext {
    pub rights_metadata_emitted: bool,
    pub hidden_marker_attempted: bool,
    pub hidden_marker_mode: HiddenMarkerMode,
    pub authentication: PayloadAuthentication,
    pub tiled: bool,
    pub progressive_jpeg_emitted: bool,
    pub output_format: ImageOutputFormat,
}
```

The payload serializer must receive this context.

Required bit semantics:

- `rights_metadata = true` only after metadata injection succeeded in the same operation;
- `hidden_marker = true` only for a payload actually being submitted to an embed path;
- metadata-only output never claims a hidden marker;
- CRC sets authentication false;
- HMAC sets authentication true;
- tiled reflects the actual selected embed path;
- progressive reflects the actual emitted JPEG representation;
- a capacity-skipped output must not later be reported as containing the attempted payload.

## 2.2 Add a canonical processing outcome

Recommended shape:

```rust
#[derive(Debug)]
pub struct ProcessingOutcome {
    pub bytes: Vec<u8>,
    pub metadata: MetadataEmissionOutcome,
    pub embed: Option<EmbedOutcomeSummary>,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone)]
pub struct EmbedOutcomeSummary {
    pub status: EmbedStatus,
    pub path: EmbedPath,
    pub payload_bytes: usize,
    pub required_capacity: usize,
    pub available_capacity: usize,
    pub effective_redundancy: usize,
    pub output_format: ImageOutputFormat,
}
```

The existing generic `EmbedOutcome<T>` may remain. Add a method that splits output from summary without discarding either:

```rust
let (bytes, summary) = embed_outcome.into_parts();
```

## 2.3 Route canonical APIs through the outcome

Update:

- `process_image_bytes` internal pipeline;
- `process_image_bytes_with_info`;
- `process_image_bytes_with_warnings`;
- `process_request_bytes`;
- `process_request_bytes_with_warnings`;
- `process_request_bytes_with_report`;
- CLI protect human output;
- CLI protect JSON output;
- strict-mode exit decision.

Compatibility APIs may discard detail at their final boundary, but the internal canonical pipeline must carry the outcome until reporting and strict decisions are complete.

Delete success inference that re-verifies final output:

```rust
stego.verify_payload_from_bytes_with_key(&result, mac_key)
    == VerificationStatus::Verified
```

`stego_succeeded` must come from the original embed result.

## 2.4 Generate the exact payload before capacity decisions

Replace fixed CRC/HMAC sizing helpers.

Example:

```rust
let payload = protector.generate_payload(&emission, ctx)?;
let payload_bits = payload
    .len()
    .checked_mul(8)
    .ok_or(Error::PayloadSizeOverflow)?;
let required_bits = payload_bits
    .checked_mul(ctx.effective_redundancy())
    .ok_or(Error::PayloadSizeOverflow)?;
```

Use actual serialized length for:

- LSB capacity;
- tiled LSB per-tile capacity;
- DCT/F5 capacity;
- tiled DCT/F5 capacity;
- preflight warnings;
- runtime warnings;
- execution reports;
- CLI JSON.

Remove `dct_required_bits_for_context` if it selects only fixed CRC/HMAC constants.

## 2.5 Required public-path tests

```text
metadata_only_has_no_hidden_payload_or_claim
marker_only_payload_reports_rights_metadata_false
metadata_and_marker_payload_reports_both_channels
crc_payload_reports_authentication_false
hmac_payload_reports_authentication_true
tiled_flag_matches_actual_embed_path
progressive_flag_matches_actual_emitted_jpeg
png_capacity_skip_propagates_to_warning_report_json_and_exit
jpeg_capacity_skip_propagates_to_warning_report_json_and_exit
progressive_qtable_only_degradation_is_not_stego_success
best_effort_returns_output_with_failed_marker_status
required_marker_capacity_skip_exits_nonzero
payload_extension_growth_changes_capacity_numbers
report_capacity_equals_embed_outcome_capacity
human_and_json_describe_same_embed_status
```

For CLI tests, parse JSON and assert exact fields rather than searching arbitrary strings.

Example expected JSON fragment:

```json
{
  "stego_attempted": true,
  "stego_succeeded": false,
  "embed": {
    "status": "skipped_capacity",
    "path": "lsb",
    "payload_bytes": 52,
    "required_capacity": 2080,
    "available_capacity": 1024
  }
}
```

Values are illustrative; tests must use the actual deterministic values produced by fixtures.

## Phase 2 acceptance criteria

- No payload channel bit is unconditionally hard-coded.
- Metadata-only processing cannot claim or generate a hidden marker.
- Actual serialized payload length drives every capacity decision.
- `EmbedOutcome` reaches warning, report, JSON, human, and strict-exit surfaces.
- Re-verification is not used as the primary embedding-success signal.
- Capacity and progressive degradations cannot be reported as successful stego.
- Human output, JSON, and exit code agree.

## Suggested commits

```text
pipeline: preserve metadata and embed outcomes
payload: derive claims and capacity from actual emission
cli: report runtime embedding outcomes consistently
```

---

# Phase 3: Enforce and report resources through an operation-local budget

## 3.1 Add an operation budget

Recommended shape:

```rust
pub(crate) struct OperationBudget<'a> {
    limits: &'a ResourceLimits,
    usage: ResourceUsage,
}

impl<'a> OperationBudget<'a> {
    pub fn new(limits: &'a ResourceLimits, input_bytes: usize) -> Result<Self>;
    pub fn observe_png_chunk(&mut self, bytes: usize) -> Result<()>;
    pub fn observe_jpeg_segment(&mut self, bytes: usize) -> Result<()>;
    pub fn observe_webp_chunk(&mut self, bytes: usize) -> Result<()>;
    pub fn observe_xmp_bytes(&mut self, bytes: usize) -> Result<()>;
    pub fn observe_xml_depth(&mut self, depth: usize) -> Result<()>;
    pub fn observe_xml_property(&mut self) -> Result<()>;
    pub fn observe_metadata_copy(&mut self, bytes: usize) -> Result<()>;
    pub fn observe_payload_probe(&mut self, bytes: usize) -> Result<()>;
    pub fn observe_payload_extract(&mut self, bytes: usize) -> Result<()>;
    pub fn observe_seed_attempt(&mut self) -> Result<()>;
    pub fn observe_tile_origin(&mut self) -> Result<()>;
    pub fn observe_manifest_record(&mut self) -> Result<()>;
    pub fn finish(self, output_bytes: usize) -> ResourceUsage;
}
```

Method names may differ. Enforcement and observation must happen together where work occurs.

## 3.2 Apply pre-work checks at public byte entrypoints

Create an inventory in `plans/030-status.md`:

```text
entrypoint | input check | header dimension check | shared budget | output usage | public test
```

Cover at minimum:

- `process_image_bytes`;
- parallel processing entrypoints;
- request APIs;
- warning/report APIs;
- metadata-only APIs;
- `verify_image_bytes`;
- `verify_image_bytes_with_limits`;
- detailed verification;
- raw-byte payload extraction;
- raw-byte payload verification;
- legal-notice verification default and limited variants;
- detached-manifest parsing;
- detached-manifest signing;
- detached-manifest verification;
- conformance manifest parsing where the public resource contract applies.

Input size must be checked before:

- metadata seed extraction;
- hashing;
- format traversal;
- image decode;
- buffer copies;
- signature iteration;
- payload probing.

Header dimensions must be checked before full decode where PNG, JPEG, or WebP headers allow it.

## 3.3 Observe real work

At production sites record:

- input and output bytes;
- PNG chunk count and bytes;
- JPEG segment count and bytes;
- WebP RIFF chunk count and bytes;
- XMP bytes inspected;
- XML depth and properties;
- metadata fields and bytes copied;
- payload prefix/header/full bytes extracted;
- seed attempts;
- tile origins;
- DCT candidate sets where bounded;
- manifest keys, signatures, and records visited;
- dimensions.

Remove or rename `peak_allocations_bytes` unless it is measured honestly. Output length is not peak allocation.

## 3.4 One bounded-failure test per limit

For every `ResourceLimits` field, add a row:

```text
limit | production enforcement function | public entrypoints | success beyond cap | failure at cap | observed counter
```

Examples:

Seed-cap proof:

```rust
let limits_one = ResourceLimits::default().with_max_verification_seeds(1);
assert_eq!(verify_with_limits(&fixture_with_seed_at_index_2, &limits_one), NotFound);

let limits_three = ResourceLimits::default().with_max_verification_seeds(3);
assert_eq!(verify_with_limits(&fixture_with_seed_at_index_2, &limits_three), Verified);
assert_eq!(usage.verification_seeds_tried, 3);
```

Tile-origin proof:

```rust
assert!(verify_with_origin_cap(&fixture, 1).is_not_found());
let result = verify_with_origin_cap(&fixture, 4);
assert!(result.is_verified());
assert_eq!(result.usage.tile_origins_checked, 4);
```

Required failure tests:

- oversized input before metadata scan;
- dimensions before full decode;
- PNG chunk count;
- JPEG segment count;
- WebP chunk count;
- XMP byte cap;
- XML depth;
- XML property count;
- metadata field count;
- metadata copy bytes;
- payload prefix/header/full extraction limits;
- seed attempts;
- tile origins;
- manifest record limits.

## Phase 3 acceptance criteria

- Every public byte entrypoint performs applicable checks before expensive work.
- One operation-local budget supplies both enforcement and reporting.
- Every limit has a production enforcement site and bounded-failure public test.
- Seed and origin caps are proven by success-beyond-cap scenarios.
- Reports contain observed work, not post-hoc estimates.
- No field labels output length as peak allocation.
- Library error, CLI human output, JSON, and exit behavior agree for limit failures.

## Suggested commits

```text
limits: introduce operation-local enforcement budget
limits: thread budget through image and stego paths
limits: thread budget through metadata and detached paths
limits: add public bounded-failure matrix
```

---

# Phase 4: Complete detached validation, caller-key semantics, and CLI adversarial coverage

## 4.1 Return immediately on invalid manifest structure

Current validation must not be followed by hashing or signature iteration.

Required control flow:

```rust
if let Err(error) = manifest.validate() {
    return ManifestVerification::invalid_configuration(error);
}

limits.check_input_size(image_bytes.len())?;
// only now hash, inspect dimensions, verify signatures, and inspect embedded reference
```

For a result-returning API, preserve structured diagnostics. Do not panic.

Use test-only counters to assert zero calls to:

- image hasher;
- signature verifier;
- image decoder;
- embedded-reference extractor.

## 4.2 Define mismatch priority explicitly

When a caller-owned key ID matches a signature record and a manifest key entry exists with different bytes:

- set `key_id_matched = true`;
- set `key_material_matched = false`;
- classify overall integrity as key-material mismatch even when the attacker signature is not valid under the caller key;
- map CLI exit to 3;
- expose a clear diagnostic.

Do not require “some cryptographically valid signature” before recognizing the contradiction between caller-owned and manifest key material.

Recommended priority:

```text
invalid manifest structure
input/resource failure
binding failure
caller-key material contradiction
signature failure
embedded-reference failure
trusted/untrusted success
```

Document the final priority in code and tests.

## 4.3 Report trust mode

JSON and human output must include:

```text
trust_mode = none | key_id_only | caller_verifying_key | callback
```

For each signature include:

- key ID;
- cryptographic validity;
- key-ID match;
- key-material match;
- trust result;
- diagnostic code when invalid.

Do not expose private or full public key bytes.

## 4.4 Add true CLI end-to-end tests

Use `assert_cmd` or the repository's existing CLI harness. Generate real keys and valid signatures.

Required matrix:

| Manifest/key state | Expected status | Exit |
|---|---|---:|
| A ID, A bytes, A signature, CLI key A | verified trusted | 0 |
| A ID, B bytes, B signature, CLI key A | key material mismatch | 3 |
| A ID, B bytes, A signature, CLI key A | key material mismatch | 3 |
| A ID, A bytes, B signature, CLI key A | signature failure | 3 |
| valid signature, no CLI key | verified untrusted | 4 |
| unrelated explicit CLI key | integrity/signature mismatch | 3 |
| wrong image | binding failure | 3 |
| malformed manifest | invalid configuration | 2 |
| duplicate key IDs | invalid configuration | 2 |
| missing payload HMAC key | embedded authentication key missing | 3 |
| wrong payload HMAC key | embedded authentication failed | 3 |

Attacker-substitution construction example:

```text
1. Generate trusted keypair A.
2. Generate attacker keypair B.
3. Create a valid claim for an image.
4. Put A's key ID in both the signature and manifest key record.
5. Put B's public bytes in the manifest key record.
6. Sign canonical claim bytes with B.
7. Run verify-manifest with A's public-key file.
8. Assert exit 3, trust_mode caller_verifying_key, key_id_matched true,
   key_material_matched false, and a key_material_mismatch diagnostic.
```

Do not use malformed base64 or invalid key length as the attacker test.

## Phase 4 acceptance criteria

- Invalid manifest structure returns before hashing, signature verification, image decode, or embedded extraction.
- Caller-key material contradiction has priority over generic signature failure.
- Caller-key mismatch always exits 3.
- Exit 4 is reserved for valid evidence without a caller trust anchor.
- JSON and human output include trust mode and key-material status.
- Every matrix row has a real CLI test.
- The CLI uses caller-owned bytes through the patch-compatible API selected in Phase 0.

## Suggested commits

```text
detached: fail invalid manifests before expensive verification
detached: prioritize caller key-material contradictions
cli: add trust-mode and key-material diagnostics
cli: add detached adversarial end-to-end matrix
```

---

# Phase 5: Correct conformance provenance, coverage, and preservation proof

## 5.1 Correct independent fixture provenance

For every independent fixture record, provide:

- actual authoring tool or script;
- exact Python version;
- exact ImageMagick version when used;
- exact generator file SHA-256 or repository commit SHA;
- exact command;
- base image provenance;
- license and redistribution terms;
- fixture SHA-256;
- expected values;
- explanation of independence from production writers.

Do not:

- claim ExifTool authored bytes when Python authored the chunks;
- use an XMP `xmptk` value as the generator version;
- say “any recent version” in an immutable provenance record.

Recommended record fields:

```toml
authoring_tool = "python-stdlib-png-chunk-writer"
authoring_tool_version = "Python 3.13.5"
auxiliary_tool = "ImageMagick convert"
auxiliary_tool_version = "ImageMagick 7.1.2-3"
generator_path = "tests/fixtures/conformance/independent/generate_independent_fixtures.py"
generator_sha256 = "..."
generation_command = "python3 tests/.../generate_independent_fixtures.py"
base_provenance = "generated 64x64 solid-color PNG; no production writer used"
source = "external"
license = "MIT"
```

If the manifest schema cannot express these fields, add compatible optional fields to the conformance-only schema.

## 5.2 Add negative coverage tests

Clone or build an in-memory fixture manifest and remove one requirement at a time.

Required tests:

```text
strict_fails_without_external_legacy
strict_fails_without_external_conflict
strict_fails_without_external_preservation
strict_fails_without_external_canonical_png
strict_fails_without_external_canonical_jpeg
strict_fails_without_external_canonical_webp
strict_fails_without_external_alternate_prefix
strict_fails_on_missing_fixture_digest
strict_fails_on_mismatched_fixture_digest
strict_fails_on_incomplete_provenance
strict_fails_on_inconsistent_tool_version
```

Do not reduce minimums to make these tests pass.

## 5.3 Test all conflict truth-table states

Keep explicit tests for:

- expected false, observed false: pass;
- expected true, observed true: pass;
- expected true, observed false: fail with `Expected conflict not detected`;
- expected false, observed true: fail with `Unexpected conflict detected`.

Use multiple legacy values in at least one fixture to prove all are inspected.

## 5.4 Add a real public preservation test

Required sequence:

1. Load the independent preservation fixture.
2. Confirm creator, rights URL, and unrelated custom XMP are present before processing.
3. Process through a public StegoEggo API or CLI using preserve-existing policy.
4. Confirm StegoEggo-owned fields update as requested.
5. Confirm independent creator, rights URL, and custom XMP remain unchanged.
6. Confirm internal extraction and external ExifTool/xmllint extraction agree.

A manifest label alone is not preservation evidence.

## Phase 5 acceptance criteria

- Provenance truthfully identifies exact tools and versions.
- The independent generator does not import or call StegoEggo production writers.
- All negative source-aware coverage tests fail for the intended reason.
- Digest and provenance validation are blocking.
- All four conflict states are tested.
- Preservation is demonstrated through a real before/after public operation.
- Nonzero external minima remain unchanged.

## Suggested commits

```text
conformance: correct independent fixture provenance
conformance: add blocking negative coverage tests
conformance: prove public preservation behavior
```

---

# Phase 6: Unify CI, RC, semver, validation, and fuzz

## 6.1 Make one script authoritative

Use `scripts/validate-release.sh` as the canonical contract. Main CI and RC should invoke it rather than duplicate drifting commands.

Support explicit modes only when necessary:

```bash
scripts/validate-release.sh --hermetic
scripts/validate-release.sh --external
scripts/validate-release.sh --all
```

`--all` must be the release evidence mode and must include every blocking check.

Canonical commands:

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
cargo semver-checks check-release
cargo test -p stegoeggo --no-default-features
cargo test -p stegoeggo --no-default-features --features signatures
cargo test -p stegoeggo --no-default-features --features detached-manifest
cargo test -p stegoeggo --no-default-features --features signatures,detached-manifest
cargo test -p stegoeggo --all-features
cargo test -p stegoeggo-cli --all-features
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict \
  --json conformance-report.json
```

## 6.2 Restore blocking semver

Delete `continue-on-error: true` from the semver job.

Do not add a workflow-level exemption for additive changes. Additive API changes should pass semver checking. If the tool reports a genuine false positive, document the exact item and use only the narrow exemption mechanism supported by the tool.

## 6.3 Align RC

RC must not use root-only variants. It must call the canonical script and additionally:

- record exact checkout SHA;
- verify clean tree;
- upload semver report;
- upload conformance report;
- upload tool versions;
- upload package inventories for every workspace package;
- upload command transcript or summary.

Install `cargo-audit` in RC or through the canonical bootstrap step.

## 6.4 Synchronize fuzz targets automatically

Create a script or test that compares:

- every `[[bin]] name` in `fuzz/Cargo.toml`;
- every target in `.github/workflows/fuzz.yml`.

Fail when either side contains an unmatched target.

For the candidate SHA, run every target and record:

```text
target | duration | run ID | URL | SHA | result | artifact
```

No earlier-SHA fuzz run qualifies.

## Phase 6 acceptance criteria

- Main CI and RC invoke the same authoritative validation logic.
- Semver is blocking.
- RC is workspace, CLI, docs, feature-matrix, audit, deny, package, semver, external, and conformance complete.
- No release-critical step uses `continue-on-error`.
- Fuzz workflow and `fuzz/Cargo.toml` are automatically synchronized.
- Every fuzz target runs on the exact candidate SHA.

## Suggested commits

```text
ci: make release validation authoritative and blocking
ci: restore blocking semver and workspace-complete RC
fuzz: enforce workflow target synchronization
```

---

# Phase 7: Correct ledgers and produce exact-SHA release evidence

## 7.1 Create or correct status files

Create or correct:

- `plans/021-status.md`
- `plans/022-status.md`
- `plans/023-status.md`
- `plans/024-status.md`
- `plans/025-status.md`
- `plans/026-status.md`
- `plans/027-status.md`
- `plans/028-status.md`
- `plans/029-status.md`
- `plans/030-status.md`

Each must use only:

```text
Disposition: OPEN | PARTIAL | SUPERSEDED | CLOSED
```

Each ledger must include:

- plan baseline SHA;
- implementation SHAs;
- criterion mapping;
- production source path;
- public test path;
- exact commands and results;
- ignored-test rationale;
- CI, fuzz, and RC run IDs and URLs;
- artifacts and inspected values;
- limitations;
- release version;
- reviewer evidence.

Correct prior overclaims, including:

- Plan 029 Workstream A marked closed despite fixed windows;
- Plan 029 Workstream C marked `CLOSED (partial)`;
- Plan 029 Workstream E marked closed despite deferred provenance and coverage;
- Plan 029 Workstream F marked closed despite non-blocking semver and RC drift.

## 7.2 Required Plan 030 matrices

`plans/030-status.md` must contain:

1. published API compatibility and semver decision table;
2. v3 path to shared-probe call-site inventory;
3. no-legacy-fallback test matrix;
4. payload emission/channel matrix;
5. `EmbedOutcome` propagation matrix;
6. public resource-entrypoint table;
7. one row per `ResourceLimits` field;
8. detached trust and exit-code matrix;
9. CLI adversarial matrix;
10. independent fixture provenance table;
11. conformance negative-coverage table;
12. CI/RC/local command-equivalence table;
13. fuzz target consistency table;
14. exact-SHA evidence table;
15. unresolved blockers section.

The unresolved blockers section must be empty before release.

## 7.3 Candidate rehearsal

Select one immutable candidate SHA only after Phases 0-6 are complete.

From a clean checkout:

1. Run `scripts/validate-release.sh --all`.
2. Obtain green main CI for that SHA.
3. Obtain green all-target fuzz for that SHA.
4. Obtain green RC for that SHA.
5. Download and inspect all artifacts.
6. Inspect each `.crate` file inventory.
7. Install the CLI from packaged artifacts or a temporary local registry.
8. Run PNG, JPEG, and WebP protect/verify smoke tests.
9. Run metadata-only, marker-only, combined, capacity-skip, and progressive-degradation cases.
10. Run detached trusted, untrusted, attacker substitution, key-material contradiction, wrong signature, wrong image, malformed manifest, duplicate key, missing payload key, and wrong payload key cases.
11. Compare human and JSON classifications and exact exit codes.
12. Record checksums and outputs in `plans/030-status.md`.

## 7.4 Publication

Only after the candidate rehearsal succeeds:

1. Confirm the chosen version is available on crates.io.
2. Create the immutable tag from the validated SHA.
3. Publish the library crate.
4. Confirm registry/index availability.
5. Publish the CLI crate against the published library version.
6. Create the GitHub release from the same tag and SHA.
7. Record crate checksums, package contents, registry references, release URL, and tag SHA.

Do not rebuild from another ref.

## 7.5 Post-publication verification

From a clean environment:

- install the published CLI;
- verify versions;
- run PNG/JPEG/WebP smoke tests;
- run caller-key substitution and key-material contradiction;
- run trusted and untrusted detached verification;
- run correct, missing, and wrong payload-key tests;
- confirm exact JSON, human, and exit behavior;
- inspect installed metadata and checksums.

## Phase 7 acceptance criteria

- Plans 021-030 have truthful committed ledgers.
- One exact SHA has green CI, fuzz, RC, semver, strict conformance, package, and smoke evidence.
- Tag, library crate, CLI crate, and GitHub release identify the same SHA/version.
- Post-publication tests pass.
- The published CLI rejects attacker key substitution with exit 3.
- `plans/030-status.md` has no unresolved blocker.

## Suggested commits

```text
plans: correct Plans 021-030 evidence ledgers
release: record exact-SHA candidate rehearsal
release: record publication and post-publication evidence
```

---

# Mandatory focused validation after each phase

## After Phase 0

```bash
cargo semver-checks check-release
cargo test -p stegoeggo --all-features detached
cargo test -p stegoeggo-cli --all-features
```

## After Phase 1

```bash
cargo test -p stegoeggo --all-features v3_
cargo test -p stegoeggo --all-features legacy
cargo test -p stegoeggo --all-features tiled
cargo test -p stegoeggo --all-features dct
```

Also run a grep-based review:

```bash
rg 'V3_CRC_PAYLOAD_BITS|V3_HMAC_PAYLOAD_BITS' src/protected/steganography.rs
```

Every remaining use must be either a test fixture, serializer constant, or legacy-independent documentation. No initial production extraction loop may remain.

## After Phase 2

```bash
cargo test -p stegoeggo --all-features outcome
cargo test -p stegoeggo --all-features capacity
cargo test -p stegoeggo --all-features channel
cargo test -p stegoeggo-cli --all-features json
cargo test -p stegoeggo-cli --all-features strict
```

Review:

```bash
rg 'verify_payload_from_bytes_with_key\(&result' src stegoeggo-cli
rg 'hidden_marker:\s*true' src
rg 'dct_required_bits_for_context' src
```

The first and third searches must return no production success-inference/capacity-policy use. Any `hidden_marker: true` result must be justified by an already-confirmed actual embed path, not generic payload generation.

## After Phase 3

```bash
cargo test -p stegoeggo --all-features resource
cargo test -p stegoeggo-cli --all-features limit
```

Review every `ResourceLimits` getter and ensure a production enforcement call and public test exist.

## After Phase 4

```bash
cargo test -p stegoeggo --all-features detached
cargo test -p stegoeggo-cli --all-features verify_manifest
```

Run the attacker-substitution CLI test independently and inspect JSON.

## After Phase 5

```bash
cargo test -p stegoeggo --all-features conformance
cargo test --test external_tools -- --ignored
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict \
  --json conformance-report.json
```

## After Phase 6

```bash
scripts/validate-release.sh --all
```

## Final candidate

Run every command in the canonical contract plus all fuzz targets.

---

# Reviewer checklist

A reviewer must answer yes to every item:

- Does the chosen release version match actual semver compatibility?
- Is `TrustPolicy` source-compatible with published `0.2.2` when targeting `0.2.3`?
- Is semver blocking in main CI and RC?
- Does initial v3 classification extract exactly six bytes?
- Is the declared header extracted and validated before the full payload?
- Does every carrier use the same prefix/header validation?
- Are fixed 36-byte and 48-byte initial v3 candidate loops absent?
- Can any v3-magic failure enter legacy decoding?
- Are auth algorithm, tag length, key ID, extensions, limits, capacity, and arithmetic validated before full extraction?
- Does exact serialized payload length drive all capacity calculations?
- Are channel bits based on actual emission?
- Can metadata-only output claim a hidden marker?
- Does `EmbedOutcome` reach all reporting and strict-decision surfaces?
- Is re-verification no longer the primary embed-success signal?
- Does every public byte entrypoint check applicable limits before expensive work?
- Are resource counters populated where work occurs?
- Are seed and origin caps proven by success-beyond-cap tests?
- Does invalid manifest structure return before hashing and signature evaluation?
- Does caller-key contradiction have explicit priority and exit 3?
- Do CLI JSON and human output include trust mode and key-material status?
- Does a real attacker-substitution CLI test fail?
- Are independent fixture provenance records exact and truthful?
- Do negative coverage tests fail when required categories are removed?
- Does preservation use a real public before/after operation?
- Do main CI, RC, and local validation share one blocking contract?
- Is fuzz-target synchronization automated?
- Are Plans 021-030 truthful and evidence-backed?
- Are CI, fuzz, RC, tag, crates, and release tied to one SHA?
- Do post-publication security tests pass?

---

# Definition of done

Plan 030 is complete only when all conditions are true:

1. The release version is semver-correct and semver checking is blocking.
2. Published `0.2.2` callers remain source-compatible if the release remains `0.2.3`.
3. Every v3 path uses a six-byte prefix, declared-header validation, and exact-length extraction.
4. No v3-magic result can enter legacy decoding.
5. Payload claims and capacity decisions reflect actual serialized and emitted evidence.
6. `EmbedOutcome` reaches warnings, reports, JSON, human output, and strict exits.
7. Every resource limit is enforced and observed through public production paths.
8. Invalid manifests fail before hashing, signature verification, image decode, or embedded extraction.
9. Caller-owned key-material contradiction is a structured integrity failure with exit 3.
10. The complete CLI adversarial matrix passes.
11. Independent fixtures have truthful provenance, negative coverage, and public preservation proof.
12. Main CI, RC, audit, deny, semver, conformance, feature tests, and fuzz are blocking and aligned.
13. Plans 021-030 contain truthful exact evidence.
14. One exact candidate SHA passes CI, all fuzz targets, RC, package inspection, and smoke tests.
15. Publication uses that SHA only.
16. Post-publication installation and security tests pass.
17. `plans/030-status.md` contains no unresolved blocker.

If any condition is absent, keep Plans 026-030 open and keep the patch release unpublished.
