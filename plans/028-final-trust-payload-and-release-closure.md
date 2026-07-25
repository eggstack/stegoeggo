# Plan 028: Final Trust, Payload, and Release Closure

Status: Ready for implementation

Baseline: `main` at `26503aef753bb440f8d94d167bfe8fbbdd50495f`

Release hold: `0.2.3` remains unreleased. Do not tag, publish, create a GitHub release, or describe Plans 026-028 as closed until every acceptance criterion in this plan is satisfied and recorded in `plans/028-status.md` with exact-SHA evidence.

## Objective

Close the remaining correctness, trust, observability, conformance, and release-evidence gaps after Plan 027 without expanding product scope.

This plan is intentionally limited to eight unresolved blockers:

1. Caller-supplied public-key bytes are not cryptographically bound to detached-manifest trust.
2. Payload-v3 extraction still begins from fixed candidate windows instead of one shared core-header-first probe across every extraction path.
3. Payload channel flags do not always describe the evidence actually emitted.
4. Structured embedding outcomes do not reach request warnings, reports, JSON, or strict CLI behavior.
5. Resource limits and usage accounting remain incomplete at public verification/extraction boundaries.
6. External legacy, conflict, and preservation conformance requirements are configured but not independently satisfied.
7. Strict conformance and semver checks are non-blocking in CI/release-candidate workflows.
8. Plans 021-028 lack complete evidence-backed status ledgers and one exact release SHA has not passed the full release contract.

## Non-goals

Do not add:

- new image formats;
- new signature algorithms;
- certificate-chain validation or a built-in trust store;
- C2PA integration;
- new steganography algorithms;
- payload-v4;
- broad performance refactors;
- CLI command redesign unrelated to the defects below;
- new publication targets.

Do not rewrite, move, or recreate the published `0.2.2` tag or artifacts.

---

## Workstream A: Bind caller-owned key bytes to detached trust

### A1. Replace key-ID-only trust for explicit public keys

The current CLI parses a caller public-key file but ultimately provides only its key ID to `TrustPolicy::TrustKeys`. Signature verification then uses public-key bytes supplied by the manifest. This permits identity substitution when an attacker controls both a manifest key record and a signature under a trusted key ID.

Introduce an explicit caller-owned key representation:

```rust
pub struct TrustedVerifyingKey {
    pub key_id: Vec<u8>,
    pub key: VerifyingKey,
}
```

Add a verification path that accepts caller-owned keys, for example:

```rust
pub enum TrustPolicy {
    TrustNone,
    TrustKeyIds(Vec<Vec<u8>>),
    TrustVerifyingKeys(Vec<TrustedVerifyingKey>),
    TrustCallback(Box<TrustCallbackFn>),
}
```

Naming may differ, but the contract must be explicit: a trusted key ID alone is not equivalent to a trusted public key.

For `verify-manifest --key`, use caller-owned verifying-key bytes, not key-ID-only trust.

### A2. Define signature verification semantics

When a caller supplies a trusted verifying key:

1. Match the signature record by `key_id`.
2. Locate the manifest public-key entry for that `key_id` if the schema requires one.
3. Require manifest key bytes to equal the caller-owned key bytes, or ignore manifest key bytes and verify directly with the caller-owned key.
4. Verify the signature over canonical claim bytes with the caller-owned key.
5. Mark the signature trusted only when both cryptographic verification and caller-key identity binding succeed.

A manifest cannot choose alternative public-key bytes under a caller-trusted key ID.

Recommended implementation:

```rust
fn verify_with_trusted_key(
    claim_bytes: &[u8],
    sig: &SignatureRecord,
    manifest_key: Option<&PublicKeyEntry>,
    trusted: &TrustedVerifyingKey,
) -> SignatureVerification {
    if sig.key_id != trusted.key_id {
        return untrusted_no_match();
    }

    if let Some(entry) = manifest_key {
        let manifest_bytes = decode_ed25519_key(entry)?;
        if manifest_bytes != trusted.key.as_bytes() {
            return key_material_mismatch();
        }
    }

    verify_signature(&trusted.key, claim_bytes, sig)
}
```

### A3. Preserve compatibility deliberately

Existing library callers that intentionally trust only key IDs may retain a key-ID-only policy, but:

- document that it does not bind external key material;
- do not use it for `verify-manifest --key`;
- do not present key-ID-only trust as equivalent to caller-key verification;
- include the trust mode in diagnostics and JSON.

### A4. Attack-focused tests

Add library and CLI tests for the real substitution attack:

1. Generate trusted key A and attacker key B.
2. Create a valid claim.
3. Build a manifest signature record whose `key_id` is A's ID.
4. Store B's public bytes under A's ID.
5. Sign canonical claim bytes with B's private key.
6. Verify using A's public-key file.
7. Assert cryptographic/trust result is non-success and CLI exit is `3`, not `0` or `4`.

Also test:

- matching A ID + matching A bytes + A signature succeeds;
- matching A ID + B bytes + A signature fails structurally or cryptographically;
- matching A ID + A bytes + B signature fails;
- wrong external key returns non-success;
- duplicate/conflicting manifest key records remain rejected before signature evaluation;
- human and JSON diagnostics identify key-material mismatch consistently.

Do not use malformed base64 in a hex field as the substitution test. The attacker fixture must be structurally valid and cryptographically self-consistent under the attacker key.

### Workstream A acceptance criteria

- `verify-manifest --key` verifies with or compares against the exact caller-supplied public-key bytes.
- A manifest cannot substitute attacker key bytes under a trusted key ID.
- Key-ID-only trust remains clearly distinguished from caller-key trust.
- The structurally valid attacker substitution test fails deterministically.
- Correct generated key files still produce exit `0`.
- No caller-owned key path relies only on manifest-controlled key bytes.

---

## Workstream B: Use one shared core-header-first payload-v3 extractor

### B1. Define one version probe contract

Replace fixed candidate iteration as the first step for v3 with a shared probe used by all extraction and verification paths.

The probe must extract only the minimum bytes required to classify the payload and read `total_length`:

```rust
pub enum PayloadProbe {
    NotV3,
    V3 {
        total_length: usize,
        auth_algorithm: AuthAlgorithm,
        auth_tag_len: usize,
    },
    MalformedV3(PayloadMalformedReason),
    UnsupportedVersion(u8),
    InsufficientCapacity,
}
```

The exact minimum probe length should derive from the wire-format constants. Do not use 48 bytes merely because that is the current fixed HMAC payload size.

### B2. Validate the v3 header before re-extraction

At probe time validate at minimum:

- magic prefix;
- version byte;
- header length;
- total length lower and upper bounds;
- total length relative to header, key-ID, extension, and auth-tag lengths;
- supported authentication algorithm;
- valid authentication-tag length for the algorithm;
- extension count and encoded extension bounds where present;
- critical/unsupported extension rules;
- `ResourceLimits::max_payload_bytes` before full extraction;
- arithmetic overflow before converting bytes to bits or multiplying redundancy.

Then extract exactly `total_length * 8` bits from the same channel/seed/path.

### B3. Prohibit legacy fallback after v3 identification

Rules:

1. If the first classified bytes are not v3 magic, legacy v2/v1 candidates may be attempted.
2. If v3 magic is present but malformed, return `MalformedV3`.
3. If v3 magic is present with an unsupported version, return `UnsupportedVersion`.
4. If the declared v3 length exceeds capacity or resource limits, return the corresponding structured result.
5. Never continue into v2/v1 decoding after any v3-magic result.

### B4. Apply the same probe to every path

Inventory every production path in `plans/028-status.md` and map it to the shared probe:

- PNG/WebP non-tiled LSB extraction;
- PNG/WebP non-tiled LSB verification;
- PNG/WebP tiled LSB extraction;
- PNG/WebP tiled LSB verification;
- JPEG baseline DCT/F5 extraction;
- JPEG baseline DCT/F5 verification;
- JPEG tiled DCT/F5 extraction;
- JPEG tiled DCT/F5 verification;
- raw-byte embedded-reference extraction;
- any known-seed or metadata-seed fallback wrappers.

Delete or stop using production loops whose v3 behavior depends on trying `V3_CRC_PAYLOAD_BITS` or `V3_HMAC_PAYLOAD_BITS` first.

Legacy v1/v2 constants may remain only for legacy extraction after `NotV3`.

### B5. Preserve v1/v2 compatibility

Keep checked-in or generated known-answer fixtures for:

- v1 CRC/ECC;
- v2 CRC/ECC;
- v2 HMAC where supported;
- each supported carrier family.

The shared probe must classify these as `NotV3`, then permit existing legacy decoding.

### B6. Difficult-area tests

Add tests for:

- v3 CRC payload whose declared length differs from the current fixed CRC constant because of a key ID or extension;
- v3 HMAC payload with a valid extension;
- v3 payload that fits the carrier but a legacy v2 window does not;
- malformed `total_length` below the core header;
- `total_length` above `max_payload_bytes`;
- auth-tag length inconsistent with algorithm;
- unsupported auth algorithm;
- unsupported future version with valid magic prefix;
- malformed v3 data that could accidentally satisfy a legacy ECC/checksum pattern;
- tiled LSB and tiled DCT paths proving no legacy decode occurs after v3 magic;
- missing HMAC key versus wrong HMAC key;
- valid v1/v2 fixtures still extracting.

Instrument test-only probe counters if needed to prove that legacy parsing was not entered after v3 identification.

### Workstream B acceptance criteria

- One shared probe implementation is used by every v3 extraction and verification path.
- Production v3 extraction does not begin with fixed CRC/HMAC payload windows.
- Exact declared length is re-extracted after bounded header validation.
- Malformed and unsupported v3 payloads never fall through to legacy decoding.
- Key IDs and extensions work without adding another fixed-size constant.
- V1/v2 compatibility tests pass.
- Payload byte limits are enforced before full extraction.

---

## Workstream C: Make payload claims and runtime outcomes truthful

### C1. Derive channel flags from actual requested emission

Do not hard-code `rights_metadata` or `hidden_marker` in `generate_payload`.

Create a payload-emission description produced by the resolved plan or pipeline:

```rust
pub struct PayloadEmissionContext {
    pub rights_metadata_emitted: bool,
    pub hidden_marker_attempted: bool,
    pub hidden_marker_mode: HiddenMarkerMode,
    pub authentication: PayloadAuthentication,
    pub progressive_jpeg: bool,
    pub tiled: bool,
}
```

The payload writer must receive values describing the actual operation being attempted:

- `rights_metadata = true` only when metadata was emitted or will be emitted in the same transaction;
- `hidden_marker = true` only when a hidden marker is actually being embedded;
- `authentication = true` only for HMAC/signature authentication, not CRC32;
- `tiled` only when the selected embed path is tiled;
- `progressive_jpeg` only when the emitted carrier is progressive JPEG.

For a marker-only request with metadata disabled, the payload must report `rights_metadata = false`.

### C2. Propagate structured embed outcomes

Extend the processing pipeline so `EmbedOutcome` survives beyond internal steganography helpers.

A suitable result shape is:

```rust
pub struct ProcessingOutcome {
    pub bytes: Vec<u8>,
    pub metadata_emitted: bool,
    pub embed: Option<EmbedOutcomeSummary>,
    pub resource_usage: ResourceUsage,
}
```

At minimum preserve:

- embedded versus skipped-capacity;
- LSB, tiled LSB, DCT, or tiled DCT path;
- generated payload bytes;
- required and available capacity;
- progressive fallback where applicable;
- actual output format.

### C3. Drive warnings and reports from execution

`process_request_bytes_with_warnings` must append runtime degradation warnings after execution.

`process_request_bytes_with_report` must derive:

- `stego_attempted` from the execution plan;
- `stego_succeeded` from `EmbedOutcome`, not output re-verification alone;
- capacity warning details from the exact attempted path;
- `metadata_injected` from the actual metadata outcome;
- payload/channel information from actual emission.

Do not infer a capacity skip only by pre-calculating image dimensions or re-verifying the final image.

### C4. Use actual generated payload size for capacity

Generate or size the exact payload before capacity checks:

```rust
let payload = generate_payload(&emission_context, ctx)?;
let required_bits = payload.len()
    .checked_mul(8)
    .and_then(|n| n.checked_mul(effective_redundancy))
    .ok_or(Error::PayloadSizeOverflow)?;
```

Apply the same value to:

- LSB capacity checks;
- tiled LSB per-tile capacity;
- DCT/F5 capacity checks;
- tiled DCT capacity;
- warnings and reports.

Do not select capacity from only the current 36-byte/48-byte constants.

### C5. Strict CLI behavior

For strict mode:

- a requested required marker that is skipped for capacity must exit nonzero;
- BestEffort may emit output, but must report the degraded result in human and JSON output;
- output must not claim a marker succeeded when `EmbedOutcome::SkippedCapacity` occurred;
- output must not claim payload-v3 evidence exists when no payload was embedded.

### C6. Tests

Add tests for:

- metadata disabled + hidden marker enabled produces payload `rights_metadata = false`;
- metadata enabled + marker enabled produces both flags true;
- CRC payload has authentication false;
- HMAC payload has authentication true;
- non-tiled/tiled and baseline/progressive flags match actual output;
- tiny PNG/WebP and low-capacity JPEG produce runtime warnings;
- request warning API, report API, CLI human output, and CLI JSON agree;
- strict CLI exits nonzero on required marker capacity skip;
- BestEffort returns output but `stego_succeeded = false` and includes capacity details;
- key ID/extension growth changes required capacity using actual payload length.

Decode and assert the actual channel and payload flag fields. Tests that check only magic/version bytes do not satisfy this workstream.

### Workstream C acceptance criteria

- No payload channel bit is hard-coded independently of actual emission intent.
- Actual generated payload length drives every capacity calculation.
- `EmbedOutcome` reaches warnings, execution reports, CLI JSON, and strict CLI behavior.
- Runtime capacity degradation cannot be reported as success.
- Channel-flag tests decode and assert the bitfields.
- Marker-only and metadata-only combinations report truthful evidence.

---

## Workstream D: Complete public-boundary resource enforcement and accounting

### D1. Enforce limits before expensive work at every byte entrypoint

Create an entrypoint table in `plans/028-status.md`:

```text
entrypoint | input-size check | header dimension check | parser budget | usage propagation | public test
```

Cover at minimum:

- `process_image_bytes`;
- `process_image_bytes_with_warnings`;
- request processing APIs;
- metadata-only APIs;
- `verify_image_bytes` and limited variants;
- stego raw-byte extraction;
- stego raw-byte verification;
- detached verification;
- manifest parsing/signing;
- conformance fixture parsing where production-limit claims are made.

Input size must be checked before hashing, metadata scanning, image decoding, copying, or traversal.

Where format headers permit it, check dimensions before a full decode.

### D2. Use one operation-local budget

Thread one operation-local object through production work:

```rust
pub struct OperationBudget<'a> {
    limits: &'a ResourceLimits,
    usage: ResourceUsage,
}
```

Record observed work at the enforcement site:

- PNG chunks and bytes;
- JPEG segments and bytes;
- WebP chunks and bytes;
- XMP bytes;
- XML depth and property visits;
- metadata fields and bytes copied;
- payload probe and extracted bytes;
- tile origins attempted;
- verification seeds attempted;
- detached manifest records visited;
- image dimensions and input/output bytes.

Return or merge the same usage object into execution and verification reports.

### D3. Remove placeholder accounting

Do not use output length as peak allocation unless the field is explicitly documented as an output-size observation.

For every public `ResourceUsage` field:

- populate it from production work;
- mark it unavailable when it cannot be observed reliably; or
- rename/document it as an estimate.

Do not report zero for work known to have occurred.

### D4. Replace success-only cap tests with enforcement tests

A test that succeeds with `max_verification_seeds = 1` does not prove only one seed was attempted.

Add observable tests that:

- force the successful seed beyond the configured cap and assert bounded failure;
- create a tiled payload where the successful origin is beyond the cap and assert bounded failure;
- compare observed counters for cap `1` versus a larger cap;
- exceed each container/metadata/XML/payload/manifest limit through a public entrypoint;
- assert stable structured errors and CLI exit classification.

### D5. Required field ledger

For every current `ResourceLimits` field, add a row:

```text
limit field | production enforcement function | public-boundary test | observed failure/result | usage counter
```

No field may be marked closed using only:

- a builder/getter assertion;
- a direct `ResourceUsage` setter test;
- a helper test that bypasses the public operation.

### Workstream D acceptance criteria

- Every public byte-processing and verification entrypoint checks applicable limits before expensive work.
- Every current limit field has a production enforcement site and public-boundary test.
- Seed and tile-origin caps are proven by bounded-failure tests and observed counters.
- Execution and verification reports contain observed production usage.
- Placeholder zeros and misleading allocation claims are removed.
- Resource failures map consistently to library errors, CLI human output, JSON, and exit codes.

---

## Workstream E: Supply genuinely independent conformance fixtures

### E1. Keep nonzero source-aware minima

Retain:

```text
external_legacy_min       >= 1
external_conflict_min     >= 1
external_preservation_min >= 1
```

Also retain external canonical PNG/JPEG/WebP and alternate-prefix requirements.

Do not reduce minima or make strict checks non-blocking to accommodate missing fixtures.

### E2. Add one independent fixture per missing category

Add at least:

- one external legacy fixture;
- one external conflict fixture;
- one external preservation fixture.

Each must be authored independently of StegoEggo production writers. Acceptable sources include:

- an exact ExifTool command/config;
- a small checked-in standards-oriented fixture generator that does not call StegoEggo metadata writers;
- a fixture from another implementation with redistribution permission.

A raw injector copied from StegoEggo production serialization is not independent.

### E3. Required provenance

For every independent fixture record:

- authoring tool and exact version;
- exact command or generator revision;
- checked-in config/script path;
- base image provenance;
- license/redistribution terms;
- SHA-256 digest;
- expected DMI/legal/conflict/preservation values;
- reason it qualifies as independent;
- regeneration limitations where byte-for-byte reproduction is platform-sensitive.

Example conflict fixture:

```text
Base PNG authored independently.
ExifTool config defines canonical and legacy DMI namespaces.
Command writes canonical=PROHIBITED and legacy=ALLOWED in one XMP packet.
Expected result documents precedence and conflict detection.
```

Example preservation fixture:

```text
ExifTool authors creator, rights URL, and unrelated custom XMP.
StegoEggo updates only its owned fields.
Test proves the externally authored creator, rights URL, and custom property survive.
```

### E4. Make platform variability explicit without disabling correctness

If JPEG byte encoding differs across tool/platform versions:

- do not require regenerated JPEG bytes to match a cross-platform digest during normal CI;
- keep the checked-in fixture digest immutable;
- run semantic extraction and provenance checks on the checked-in fixture;
- separate optional regeneration verification from strict conformance;
- document exact tool version used to create the checked-in artifact.

This is not a reason to mark semantic strict conformance `continue-on-error`.

### E5. Negative coverage tests

Add tests that remove each independent category from an in-memory manifest and assert strict coverage fails for:

- external legacy;
- external conflict;
- external preservation;
- external canonical format coverage;
- external alternate prefix.

### Workstream E acceptance criteria

- Nonzero external minima are satisfied by actual independent records.
- Each missing category has at least one independently authored fixture.
- Provenance and reproduction information is complete and truthful.
- Strict coverage fails when any required independent category is removed.
- Platform-specific regeneration concerns do not disable semantic conformance.

---

## Workstream F: Restore blocking CI and release-candidate gates

### F1. Use one blocking validation contract

Update CI, release-candidate workflow, publication preflight, and `scripts/validate-release.sh` to run equivalent commands:

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

Explicit feature validation:

```bash
cargo test -p stegoeggo --no-default-features
cargo test -p stegoeggo --no-default-features --features signatures
cargo test -p stegoeggo --no-default-features --features detached-manifest
cargo test -p stegoeggo --no-default-features --features signatures,detached-manifest
cargo test -p stegoeggo --all-features
cargo test -p stegoeggo-cli --all-features
```

### F2. Make strict conformance blocking

Remove `continue-on-error: true` from strict conformance in:

- main CI;
- release-candidate workflow;
- any publication preflight.

The exact strict command must fail the job when semantic conformance, fixture digest verification, provenance validation, or coverage minimums fail.

### F3. Resolve semver correctly

Do not keep semver checking non-blocking merely because additive API changes exist.

For `0.2.3`:

- additive APIs should pass semver checks;
- accidental breaking changes must be fixed or deferred;
- intentionally retained compatibility wrappers must be tested;
- record the semver baseline and report artifact.

Remove `continue-on-error: true` from the release-candidate semver step once the tool invocation is correct for the workspace/package layout.

If `cargo-semver-checks` has a documented false positive, add a narrowly documented exemption rather than ignoring the entire step.

### F4. Explicit CLI and workspace jobs

Ensure CI visibly blocks on:

- workspace clippy;
- workspace all-feature tests;
- CLI all-feature integration tests;
- docs;
- package workspace;
- security/license checks;
- strict conformance;
- external integration.

Root-package success is insufficient.

### F5. Fuzz exact release SHA

Enumerate every fuzz target and run release-SHA smoke coverage in the supported nightly fuzz workflow.

Record:

- target list;
- commands/durations;
- run ID/URL;
- exact SHA;
- crashes or corpus changes.

### Workstream F acceptance criteria

- Strict conformance is blocking in CI and RC.
- Semver checking is blocking or has only narrowly documented per-item exemptions.
- Workspace and CLI commands match the plan contract.
- Every required feature combination is tested.
- All fuzz targets run against the exact candidate SHA.
- No release-critical job relies on `continue-on-error`.

---

## Workstream G: Create truthful closure ledgers

### G1. Create or correct status files

Create or correct:

- `plans/021-status.md`
- `plans/022-status.md`
- `plans/023-status.md`
- `plans/024-status.md`
- `plans/025-status.md`
- `plans/026-status.md`
- `plans/027-status.md`
- `plans/028-status.md`

### G2. Required ledger structure

Each file must include:

- final disposition: open, partially complete, superseded, or closed;
- plan baseline SHA;
- implementation SHAs;
- acceptance-criterion mapping;
- exact local commands and results;
- test counts and ignored-test explanations;
- CI, fuzz, and RC run IDs/URLs;
- artifacts and inspected values;
- known limitations/non-goals;
- release version containing the work;
- reviewer sign-off or evidence reference.

### G3. Evidence rules

- Commit messages are not evidence.
- A test name is not evidence that it executed.
- A non-blocking workflow step is not closure evidence.
- Local claims require exact command output recorded or attached.
- CI evidence must identify the exact SHA.
- Status files must not claim publication before registry availability and post-publication smoke verification.

### G4. Plan 028 field tables

`plans/028-status.md` must additionally contain:

1. v3 extraction path inventory and shared-probe mapping;
2. trust/key-binding test matrix;
3. payload flag combination matrix;
4. `EmbedOutcome` propagation matrix;
5. public entrypoint/resource-enforcement table;
6. one row per `ResourceLimits` field;
7. independent fixture inventory and provenance links;
8. CI/RC/fuzz exact-SHA evidence table;
9. unresolved-item section, which must be empty before release.

### Workstream G acceptance criteria

- Plans 021-028 have committed status ledgers.
- Every closed criterion cites code, test, command, run, or artifact evidence.
- Historical claims are corrected where implementation diverged from earlier commit messages.
- `plans/028-status.md` contains all required matrices and no unresolved blocker before publication.

---

## Workstream H: Exact-SHA release and post-publication closure

### H1. Keep the version provisional

`0.2.3` remains an unreleased candidate until all prior workstreams pass.

Before RC:

- confirm `0.2.3` has not already been published or reserved;
- otherwise choose the next patch version and update every workspace/package/reference consistently;
- keep the changelog heading `Unreleased` until publication;
- ensure the `0.2.2` heading uses its actual release date.

### H2. Exact-SHA rehearsal

For one final candidate SHA:

1. Clean checkout.
2. Run the blocking validation script.
3. Obtain green main CI.
4. Obtain green all-target fuzz smoke.
5. Run release-candidate validation for that exact SHA.
6. Inspect all artifacts.
7. Unpack `.crate` archives and inspect file inventories.
8. Install the CLI from the packaged artifact or temporary local registry.
9. Run PNG, JPEG, and WebP protect/verify smoke tests.
10. Run detached `keygen -> sign -> verify-manifest` smoke tests.
11. Run attacker key-substitution, untrusted, wrong-key, wrong-image, malformed-manifest, missing-payload-key, and wrong-payload-key cases.
12. Verify exact exit codes and JSON/human agreement.
13. Record all evidence in `plans/028-status.md`.

### H3. Publication sequencing

Only after exact-SHA RC evidence is complete:

1. Create the immutable release tag from the validated SHA.
2. Publish the library crate.
3. Confirm registry/index availability.
4. Publish the CLI against the published library version.
5. Create the GitHub release from the same SHA/tag.
6. Record crate checksums, package contents, and registry URLs.

Do not rebuild or publish from a different ref.

### H4. Post-publication verification

From a clean environment:

- install the library/CLI from crates.io;
- verify reported versions;
- run minimal PNG/JPEG/WebP protection and verification;
- run trusted and untrusted detached verification;
- run correct/missing/wrong payload-key cases;
- confirm attacker key substitution fails;
- inspect installed package metadata;
- record checksums and observed outputs.

### Workstream H acceptance criteria

- One exact SHA has green CI, fuzz, strict conformance, semver, RC, and package evidence.
- Tag, library crate, CLI crate, and GitHub release all identify that SHA/version.
- Post-publication installation and smoke tests pass.
- The attacker substitution test still fails in the published CLI.
- `plans/028-status.md` records all release and post-release evidence.

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

Also run:

- every feature combination in Workstream F;
- strict conformance with nonzero independent source-aware minima;
- all fuzz targets;
- the full attack-focused trust suite;
- payload-v3 exact-length tests across all extraction paths;
- channel-flag and execution-outcome matrices;
- resource-limit public-boundary tests;
- packaged and post-publication CLI smoke cases.

Record exact commands and conclusions, not only aggregate test counts.

---

## Recommended implementation order

1. Workstream A: close the detached trust vulnerability first.
2. Workstream B: centralize and prove v3 probe behavior.
3. Workstream C: make payload claims and runtime outcomes truthful.
4. Workstream D: finish resource enforcement and accounting.
5. Workstream E: add independent fixtures and strict semantic coverage.
6. Workstream F: restore all blocking gates.
7. Workstream G: complete evidence ledgers.
8. Workstream H: execute exact-SHA RC, publication, and post-publication verification.

Do not begin publication work while A-F contain an unresolved criterion.

---

## Reviewer checklist

A reviewer must answer yes to every item:

- Does `verify-manifest --key` bind trust to caller-owned public-key bytes?
- Can a structurally valid attacker key under a trusted key ID ever produce success?
- Is key-ID-only trust clearly distinguished from caller-key trust?
- Does every v3 path call one shared core-header-first probe?
- Are fixed v3 CRC/HMAC windows absent from the initial production classification path?
- Does any v3-magic failure prevent legacy fallback?
- Do key IDs/extensions work without a new fixed-size constant?
- Do v1/v2 fixtures still pass?
- Do payload channel flags match actual metadata, marker, authentication, tiled, and progressive emission?
- Does actual generated payload length drive capacity?
- Does `EmbedOutcome` reach warnings, reports, JSON, and strict exits?
- Does a required capacity skip fail strict mode?
- Does every public verification/extraction entrypoint check limits before expensive work?
- Are tile-origin and seed caps demonstrated through bounded failure and counters?
- Are usage counters populated by production operations?
- Are external legacy/conflict/preservation fixtures genuinely independent?
- Is strict conformance blocking?
- Is semver checking blocking or narrowly exempted?
- Do workspace and CLI jobs use the required commands?
- Do Plans 021-028 have truthful evidence-backed ledgers?
- Are CI, fuzz, RC, tag, crates, and release tied to one SHA?
- Do post-publication smoke tests include the attacker key-substitution case?

---

## Definition of done

Plan 028 is complete only when:

1. Caller-owned public-key bytes are cryptographically bound to detached trust.
2. Every payload-v3 path uses one bounded header-first exact-length probe with no malformed-v3 legacy fallback.
3. Payload flags and capacity calculations describe actual emitted evidence.
4. Structured embedding outcomes reach all public warnings/reports/CLI surfaces.
5. Every resource limit is enforced and tested through public production boundaries with observed accounting.
6. Independent external legacy, conflict, and preservation fixtures satisfy strict source-aware coverage.
7. Strict conformance, semver, workspace, CLI, security, and package checks are blocking and green.
8. Plans 021-028 status ledgers are committed and evidence-backed.
9. One exact SHA passes main CI, all-target fuzz smoke, and release-candidate validation.
10. The immutable patch release is published from that SHA.
11. Post-publication installation and all image/detached/security smoke tests pass.
12. `plans/028-status.md` contains no unresolved blocking item.

If any condition is absent, keep Plans 026-028 open and do not describe this line of work as complete.