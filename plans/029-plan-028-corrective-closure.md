# Plan 029: Plan 028 Corrective Closure Pass

Status: Ready for implementation

Baseline: `main` at `533b68d1f7d410f6bb70b1366c5348ef48278bbe`

Supersedes: Plan 028 implementation claims that are not demonstrated by the current production code or exact-SHA evidence.

Release hold: `0.2.3` remains unreleased. Do not tag, publish, create a GitHub release, or mark Plans 026-029 closed until every blocking criterion in this plan is satisfied and recorded against one exact candidate SHA.

## Objective

Close the remaining Plan 028 defects with the smallest coherent corrective pass. This is not a new feature phase. It is a correctness, observability, conformance, and release-evidence closure pass.

The implementation must correct seven unresolved areas:

1. Payload-v3 extraction still starts from fixed 36-byte/48-byte candidate windows and does not use one shared production header probe.
2. Payload channel flags, capacity calculations, warnings, reports, JSON, and strict CLI behavior are not derived from the actual execution outcome.
3. Resource limits and usage accounting are not consistently enforced or observed at public byte-processing and verification boundaries.
4. Caller-owned key binding exists, but key-material mismatch classification, CLI diagnostics, exit behavior, validation ordering, and CLI adversarial coverage remain incomplete.
5. Independent conformance fixtures have inconsistent provenance, incomplete negative coverage, and a conflict-expectation regression.
6. CI, release-candidate validation, and `scripts/validate-release.sh` do not execute one equivalent blocking contract.
7. Plans 021-029 lack complete evidence ledgers, and no exact candidate SHA has completed CI, fuzz, RC, package, publication, and post-publication validation.

## Non-goals

Do not add:

- payload-v4;
- new steganography algorithms;
- new image formats;
- new signature algorithms;
- certificate-chain validation;
- a built-in trust store;
- C2PA integration;
- broad performance refactors;
- unrelated CLI redesign;
- new metadata namespaces unless required to correct the existing conformance fixtures;
- new release targets.

Do not change, recreate, or move the published `0.2.2` tag or artifacts.

---

## Closure discipline

This plan cannot be closed through comments, changelog text, commit messages, or status-ledger assertions alone.

For every criterion, the implementation must provide all applicable evidence:

- production implementation location;
- public or production-path test;
- exact command executed;
- observed result;
- CI/RC run tied to the exact implementation SHA;
- artifact or JSON output where relevant.

A private helper test is insufficient when the requirement governs a public API or CLI path.

A test that merely succeeds under a configured cap is insufficient to prove the cap bounded work.

A fixed-size v3 candidate loop is not a header-first probe even when it later reads `total_length`.

A re-verification of output is not a substitute for propagating the original `EmbedOutcome`.

---

# Workstream A: Replace fixed-window v3 extraction with one shared production probe

## A1. Define the actual minimum v3 probe contract

Create one shared probe result used by every v3 extraction and verification path. The type may differ, but it must represent at least:

```rust
pub enum PayloadProbe {
    NotV3,
    V3 {
        header_length: usize,
        total_length: usize,
        auth_algorithm: AuthAlgorithm,
        auth_tag_len: usize,
        extension_count: usize,
    },
    MalformedV3(PayloadMalformedReason),
    UnsupportedVersion(u8),
    InsufficientCapacity,
    ResourceLimitExceeded,
}
```

Derive the minimum number of bytes required for classification from the v3 wire-format constants. Do not define the probe as 36 or 48 bytes. Do not use the present maximum authentication-tag length as the initial probe size unless the wire format genuinely requires those bytes to determine total length.

The probe must read only enough bytes to classify the payload and determine the exact bounded extraction length.

## A2. Validate the header before full extraction

Before extracting `total_length` bytes, validate:

- complete magic prefix;
- version byte;
- declared header length;
- declared total length;
- `total_length >= header_length`;
- core fields fit within the declared header;
- authentication algorithm is supported;
- authentication-tag length is valid for that algorithm;
- key-ID length and extension encoding fit within the header;
- extension count and cumulative extension lengths are bounded;
- critical unknown extensions fail deterministically;
- `total_length <= ResourceLimits::max_payload_bytes`;
- carrier capacity is sufficient for `total_length * 8` and effective redundancy;
- all byte-to-bit and redundancy arithmetic uses checked operations.

Do not allocate or extract the full payload before these checks pass.

## A3. Enforce no legacy fallback after v3 identification

The production contract is:

1. If the initial bytes are not v3 magic, legacy v2/v1 decoding may run.
2. If v3 magic is present and the version is unsupported, return `UnsupportedVersion`.
3. If v3 magic is present and the header is malformed, return `MalformedV3`.
4. If v3 magic is present and limits or capacity fail, return the corresponding structured failure.
5. Never call legacy ECC, v2, or v1 decoding after any v3-magic classification.

This rule must hold in extraction and verification paths, including tiled paths.

## A4. Apply the same probe everywhere

Inventory and convert every production path:

- PNG/WebP non-tiled LSB extraction;
- PNG/WebP non-tiled LSB verification;
- PNG/WebP tiled LSB extraction;
- PNG/WebP tiled LSB verification;
- JPEG baseline DCT/F5 extraction;
- JPEG baseline DCT/F5 verification;
- JPEG tiled DCT/F5 extraction;
- JPEG tiled DCT/F5 verification;
- raw-byte embedded-reference extraction;
- raw-byte embedded-reference verification;
- metadata-seed wrappers;
- fixed-seed/test-seed wrappers used by public verification;
- any detached-manifest embedded-reference path.

Delete or remove from production use:

- initial loops over `V3_CRC_PAYLOAD_BITS` and `V3_HMAC_PAYLOAD_BITS`;
- tiled lists that attempt v2/v1 before v3 classification;
- dead `V3ProbeResult` implementations not used by production;
- comments or changelog claims that describe fixed-window probing as header-first extraction.

Legacy fixed-size constants may remain only inside legacy decoding after `NotV3`.

## A5. Exact-length extraction helper

Introduce channel-specific helpers with a common semantic contract, for example:

```rust
fn probe_then_extract<C: BitChannel>(
    channel: &C,
    location: ExtractionLocation,
    limits: &ResourceLimits,
) -> CandidateOutcome
```

The helper must:

1. extract the minimum probe bits;
2. classify and validate the header;
3. extract exactly `total_length * 8` bits from the same seed, pass, tile, coefficient set, or origin;
4. verify integrity/authentication;
5. return a structured result without trying legacy decoding after v3 identification.

Avoid copy-pasting slightly different v3 logic into LSB, DCT, and tiled paths. Channel-specific bit access may differ; classification and length validation must be shared.

## A6. Required adversarial and compatibility tests

Add tests covering all carrier families and extraction modes:

- valid CRC v3 with a key ID or extension that changes total length beyond 36 bytes;
- valid HMAC v3 with an extension that changes total length beyond 48 bytes;
- v3 total length below core/header minimum;
- v3 total length above `max_payload_bytes`;
- v3 total length above carrier capacity;
- unsupported v3 authentication algorithm;
- invalid auth-tag length for CRC;
- invalid auth-tag length for HMAC;
- unsupported future version with valid magic;
- malformed extension length;
- excessive extension count;
- checked-arithmetic overflow path;
- malformed v3 bytes that would satisfy a legacy checksum or ECC pattern if fallback occurred;
- tiled LSB malformed v3 proving no legacy decode call;
- tiled DCT malformed v3 proving no legacy decode call;
- missing HMAC key;
- wrong HMAC key;
- valid v1 fixtures still decode after `NotV3`;
- valid v2 fixtures still decode after `NotV3`.

Use test-only instrumentation where necessary to count probe, full extraction, and legacy-decoder entries. Assert legacy-decoder count is zero after v3 magic.

## Workstream A acceptance criteria

- One shared v3 classification and length-validation implementation is used by every production v3 path.
- The initial production extraction does not use fixed 36-byte or 48-byte v3 candidates.
- Tiled paths do not try v2/v1 before v3 classification.
- Full extraction uses the exact bounded declared length.
- Malformed, unsupported, oversized, and insufficient-capacity v3 payloads never enter legacy decoding.
- Key IDs and extensions work without adding another fixed payload-size constant.
- V1/v2 compatibility tests remain green.
- `max_payload_bytes` is enforced before full payload extraction.

---

# Workstream B: Make payload claims, capacity, and runtime reporting truthful

## B1. Introduce an explicit emission context

Do not derive payload claims from generic `ProtectionContext` fields that represent configuration rather than actual execution.

Create an explicit value produced by the resolved operation, for example:

```rust
pub struct PayloadEmissionContext {
    pub rights_metadata_emitted: bool,
    pub hidden_marker_attempted: bool,
    pub hidden_marker_mode: HiddenMarkerMode,
    pub authentication: PayloadAuthentication,
    pub tiled: bool,
    pub progressive_jpeg: bool,
    pub output_format: ImageOutputFormat,
}
```

The payload serializer must receive this value.

Required semantics:

- `rights_metadata` is true only when rights metadata was actually emitted in the same successful transaction;
- `hidden_marker` is true only when a hidden payload is being embedded;
- CRC does not set the authentication channel bit;
- HMAC does set the authentication channel bit;
- `tiled` reflects the selected execution path;
- `progressive_jpeg` reflects the actual emitted JPEG form, not merely the requested option;
- metadata-only processing must not generate or claim a hidden marker;
- a capacity-skipped output must not claim embedded v3 evidence.

## B2. Propagate `EmbedOutcome` through the canonical pipeline

Create one processing result that carries output plus observed execution state:

```rust
pub struct ProcessingOutcome {
    pub bytes: Vec<u8>,
    pub metadata: MetadataEmissionOutcome,
    pub embed: Option<EmbedOutcomeSummary>,
    pub resource_usage: ResourceUsage,
}
```

At minimum, `EmbedOutcomeSummary` must preserve:

- embedded;
- skipped for capacity;
- unsupported progressive/Q-table-only degradation;
- path: LSB, tiled LSB, DCT/F5, tiled DCT/F5;
- payload bytes;
- required capacity;
- available capacity;
- effective redundancy;
- actual output format.

Internal convenience methods may convert to raw bytes, but the canonical request and CLI paths must not discard the outcome.

## B3. Drive warnings and reports from the observed outcome

Update:

- `process_request_bytes_with_warnings`;
- `process_request_bytes_with_report`;
- CLI human output;
- CLI JSON output;
- strict-mode exit selection.

Required behavior:

- append runtime warnings after execution;
- derive `stego_succeeded` directly from `EmbedOutcome`;
- do not infer success by re-verifying output;
- include exact path and capacity values in degradation diagnostics;
- derive `metadata_injected` from the metadata emission result;
- ensure human output, JSON, and exit code describe the same result;
- strict required-marker mode exits nonzero for capacity skip or progressive unsupported degradation;
- BestEffort may return output but must report `stego_succeeded = false` and the exact degradation.

## B4. Use the actual serialized payload length everywhere

Generate or size the exact payload before capacity decisions:

```rust
let payload = generate_payload(&emission_context, ctx)?;
let required_bits = payload
    .len()
    .checked_mul(8)
    .and_then(|n| n.checked_mul(effective_redundancy))
    .ok_or(Error::PayloadSizeOverflow)?;
```

Use this value for:

- non-tiled LSB capacity;
- tiled LSB per-tile capacity;
- DCT/F5 capacity;
- tiled DCT capacity;
- warnings;
- reports;
- CLI JSON;
- tests.

Remove `dct_required_bits_for_context` and equivalent helpers if they select only CRC/HMAC fixed constants. Fixed constants may remain as wire-format fixtures but not as capacity policy.

## B5. Required tests

Add public-path tests for:

- metadata-only request produces no hidden payload and no hidden-marker claim;
- marker-only request has `rights_metadata = false`;
- metadata plus marker has both channel bits true;
- CRC has authentication false;
- HMAC has authentication true;
- tiled flag matches actual tiled path;
- progressive flag matches actual emitted JPEG behavior;
- requested progressive JPEG that degrades does not falsely claim successful hidden evidence;
- tiny PNG capacity skip reaches warning API, report API, human CLI, JSON CLI, and strict exit;
- low-capacity JPEG skip reaches the same surfaces;
- BestEffort returns output and explicit failed marker status;
- required marker returns nonzero;
- key-ID/extension payload growth increases reported required capacity;
- warning/report/JSON capacity numbers equal `EmbedOutcome` numbers;
- no code path needs to re-verify final output to determine whether embedding succeeded.

Decode and assert actual payload bitfields. Tests checking only magic/version do not qualify.

## Workstream B acceptance criteria

- No channel bit is hard-coded independently of observed emission.
- `hidden_marker` is not universally true in payload generation.
- Actual serialized payload length drives every capacity calculation.
- `EmbedOutcome` reaches warnings, reports, human output, JSON, and strict exit behavior.
- Re-verification is not used as the primary source of embedding success.
- Capacity or progressive degradation cannot be reported as successful stego.
- Metadata-only, marker-only, and combined operations produce truthful claims.

---

# Workstream C: Complete resource-limit enforcement and observed accounting

## C1. Introduce one operation-local budget

Thread a mutable operation budget through parser, extraction, verification, and reporting work:

```rust
pub struct OperationBudget<'a> {
    limits: &'a ResourceLimits,
    usage: ResourceUsage,
}
```

The budget must enforce and record work where the work occurs. Do not reconstruct usage from output size after processing.

## C2. Enforce limits before expensive work at every public byte entrypoint

Create an entrypoint inventory in `plans/029-status.md` with columns:

```text
entrypoint | pre-input check | header dimension check | parser budget | output usage propagation | public test
```

Cover at minimum:

- `process_image_bytes` and limited variants;
- request processing APIs;
- warning/report request APIs;
- metadata-only byte APIs;
- `verify_image_bytes`;
- `verify_image_bytes_with_limits`;
- detailed verification APIs;
- raw-byte stego extraction;
- raw-byte stego verification;
- detached-manifest verification;
- detached-manifest parsing and signing;
- conformance manifest parsing when production resource claims apply.

Input size must be checked before:

- hashing;
- metadata scanning;
- image decoding;
- buffer copying;
- chunk/segment traversal;
- seed extraction;
- stego probing.

Where headers permit it, dimensions must be checked before full decode.

Default public entrypoints must apply default limits; limited entrypoints must apply caller-supplied limits.

## C3. Record observed production work

Record at enforcement sites:

- input bytes;
- output bytes;
- PNG chunks and chunk bytes;
- JPEG segments and segment bytes;
- WebP RIFF chunks and bytes;
- XMP bytes inspected;
- XML depth;
- XML property visits;
- metadata fields copied;
- metadata bytes copied;
- payload probe bytes;
- full payload bytes extracted;
- verification seeds attempted;
- tile origins attempted;
- DCT tile/coefficient candidate sets attempted where bounded;
- detached manifest records visited;
- image width and height;
- any allocation estimate, clearly labeled as observed or estimated.

Do not use output length as “peak allocation.” If peak allocation cannot be measured reliably, remove, rename, or mark the field unavailable.

Do not report zero for known work.

## C4. Add one ledger row per limit field

For every current `ResourceLimits` field, add a row in `plans/029-status.md`:

```text
limit | enforcement function | entrypoints covered | public test | bounded failure | usage counter
```

No limit may be closed using only a builder/getter test or a direct `ResourceUsage` setter test.

## C5. Required bounded-failure tests

Add public-path tests that:

- place the successful verification seed beyond cap 1 and assert failure at cap 1, success at a larger cap, and observed attempt counts;
- place the successful tiled origin beyond cap 1 and assert the same pattern;
- exceed input bytes before metadata scan/hash/decode;
- exceed dimensions before full decode where supported;
- exceed PNG chunk count;
- exceed JPEG segment count;
- exceed WebP chunk count;
- exceed XMP bytes;
- exceed XML depth;
- exceed XML property visits;
- exceed metadata field/byte limits;
- exceed payload probe/full extraction bytes;
- exceed manifest key/signature/record limits;
- verify stable structured error and CLI classification.

Tests that merely succeed with cap 1 do not qualify.

## Workstream C acceptance criteria

- Every public byte-processing or verification entrypoint performs applicable pre-work checks.
- One operation-local budget supplies both enforcement and reporting.
- Every limit field has a production enforcement site and public-boundary bounded-failure test.
- Seed and tile-origin caps are proven by success-beyond-cap scenarios and counters.
- Reports contain observed production usage rather than reconstructed placeholders.
- No misleading peak-allocation claim remains.
- Library errors, human CLI output, JSON, and exit behavior agree for limit failures.

---

# Workstream D: Finish detached caller-key semantics and CLI security closure

## D1. Make key-material mismatch an integrity failure

When `TrustPolicy::TrustVerifyingKeys` is used and a signature record matches the caller’s key ID but the manifest key bytes differ from the caller-owned key:

- mark `key_material_matched = false`;
- do not classify the result as merely valid-but-untrusted;
- expose a structured key-material mismatch failure;
- map CLI result to exit `3`, not exit `4`;
- preserve exit `4` for cryptographically valid evidence that is not trusted because no matching caller trust anchor was supplied.

A manifest that contradicts an explicitly supplied key file is an integrity/configuration failure, not ordinary absence of trust.

## D2. Stop evaluation after invalid manifest structure

Perform shared manifest validation before:

- hashing the image;
- iterating signatures;
- loading image dimensions;
- embedded-reference extraction.

If validation fails, return `InvalidConfiguration` immediately with structured diagnostics. Do not evaluate signatures from duplicate/conflicting key records and then merely mark the aggregate manifest invalid afterward.

## D3. Expose trust mode and key-material status

Add to CLI JSON and human output:

- trust mode: none, key-id-only, caller-verifying-key, callback where representable;
- per-signature `key_id_matched`;
- per-signature `key_material_matched`;
- per-signature cryptographic validity;
- per-signature trust result;
- explicit mismatch diagnostic.

Do not expose private key material or full sensitive key bytes.

## D4. CLI adversarial tests

Add true CLI end-to-end tests:

1. Generate trusted key A and attacker key B.
2. Build a structurally valid claim and manifest.
3. Place B public bytes under A key ID.
4. Sign canonical claim bytes with B private key.
5. Verify using A public-key file.
6. Assert exit `3`.
7. Assert JSON contains `key_material_matched: false` or equivalent structured mismatch.
8. Assert human output identifies key-material mismatch.

Also test:

- A ID + A bytes + A signature exits 0;
- A ID + B bytes + A signature exits 3;
- A ID + A bytes + B signature exits 3;
- no key supplied with valid signature exits 4;
- unrelated caller key exits 3 when an explicit key was supplied for the matching identity contract;
- malformed manifest exits 2 before signature evaluation;
- duplicate public-key IDs exit 2 before signature evaluation;
- wrong image exits 3;
- JSON and human classifications agree.

## D5. Compatibility

Retain `TrustKeys` only as explicitly documented legacy key-ID-only trust. Do not use it in `verify-manifest --key`.

Do not silently change legacy library API signatures. Add a structured status enum or compatible extension as needed.

## Workstream D acceptance criteria

- Caller-key mismatch is exit 3 and never exit 4.
- Exit 4 means valid evidence lacking a caller trust anchor, not contradictory key material.
- Invalid manifest structure returns before hashing, signature iteration, image decode, or embedded extraction.
- CLI JSON and human output expose trust mode and key-material match status.
- A structurally valid attacker-substitution CLI test fails deterministically.
- Legacy key-ID-only APIs remain clearly separated and documented.

---

# Workstream E: Correct independent conformance and provenance

## E1. Correct fixture provenance records

For each independent fixture, record accurately:

- actual authoring tool or script;
- exact tool versions used to create the checked-in bytes;
- exact generator revision or repository SHA;
- exact command;
- base image provenance;
- license and redistribution terms;
- SHA-256 digest;
- expected internal and external values;
- reason it is independent from StegoEggo production writers.

Do not claim ExifTool authorship when the fixture was written by a Python raw PNG/XMP injector.

Do not use an XMP `xmptk` string as the authoring tool version.

Do not specify “any recent version” for a checked-in reproducibility record. Either pin the exact version used or clearly separate immutable checked-in fixture validation from optional regeneration.

## E2. Preserve genuine independence

The independent generator must not:

- import StegoEggo;
- call StegoEggo binaries;
- copy StegoEggo production serialization functions;
- share a helper with the production metadata writer.

A small standards-oriented Python generator is acceptable if its format logic is independently authored and documented.

## E3. Fix conflict expectation logic

Correct conformance behavior for all four cases:

| Expected | Observed | Result |
|---|---:|---|
| false | false | pass |
| true | true | pass |
| true | false | fail: expected conflict not observed |
| false | true | fail: unexpected conflict observed |

Correct the current reversed failure message.

Conflict detection must inspect all relevant canonical and legacy values, not only the first legacy entry when multiple legacy values can exist.

## E4. Add negative coverage tests

Programmatically remove or reclassify records from an in-memory manifest and assert strict coverage failure for:

- external legacy minimum;
- external conflict minimum;
- external preservation minimum;
- external canonical PNG;
- external canonical JPEG;
- external canonical WebP;
- external alternate-prefix coverage;
- missing fixture digest;
- mismatched fixture digest;
- incomplete provenance;
- inconsistent authoring tool/version fields.

Do not reduce source-aware minima to make tests pass.

## E5. Semantic preservation test

For the external preservation fixture:

1. Verify the independently authored creator, rights URL, and unrelated custom XMP are present before StegoEggo processing.
2. Run StegoEggo metadata update through a public API or CLI.
3. Verify StegoEggo-owned fields changed as intended.
4. Verify independent creator, rights URL, and custom property remain.
5. Verify internal and external extraction agree.

A fixture labeled preservation without an actual before/after public processing test is insufficient.

## Workstream E acceptance criteria

- Provenance records truthfully identify actual tools and exact versions.
- Independent fixtures remain independent from production writers.
- All four conflict expectation states are tested and correct.
- Strict coverage fails when any required external category or format is removed.
- Digest and provenance validation are blocking.
- The preservation fixture proves unrelated external XMP survives a public StegoEggo update.
- Nonzero external minima remain unchanged and pass.

---

# Workstream F: Unify blocking CI, RC, validation, and fuzz contracts

## F1. Define one canonical validation script

Make `scripts/validate-release.sh` the authoritative command contract, or create one equivalent script used by all workflows.

It must execute, at minimum:

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
```

Feature tests:

```bash
cargo test -p stegoeggo --no-default-features
cargo test -p stegoeggo --no-default-features --features signatures
cargo test -p stegoeggo --no-default-features --features detached-manifest
cargo test -p stegoeggo --no-default-features --features signatures,detached-manifest
cargo test -p stegoeggo --all-features
cargo test -p stegoeggo-cli --all-features
```

Strict conformance:

```bash
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict \
  --json conformance-report.json
```

If external dependencies are intentionally split into a separate phase, the default release invocation must include them. `--skip-external` may exist for local development but cannot produce release evidence.

## F2. Make main CI and RC call the same contract

Main CI and release-candidate workflows must not drift into different root/workspace command sets.

Preferred approach:

- shared shell script or reusable workflow;
- main CI invokes the hermetic and external phases;
- RC invokes the entire release contract;
- artifacts are uploaded with exact SHA metadata.

At minimum, RC must include:

- workspace clippy;
- workspace all-feature tests;
- explicit CLI tests;
- workspace docs;
- feature matrix;
- package workspace;
- deny licenses/advisories;
- audit;
- semver;
- external integration;
- strict conformance.

No release-critical step may use `continue-on-error`.

## F3. Semver handling

Use a correct package/workspace invocation and a recorded published baseline.

If a genuine `cargo-semver-checks` false positive occurs:

- identify the exact item;
- link the tool issue or document the mismatch;
- add the narrowest possible exemption;
- retain the remainder of semver checking as blocking.

Do not ignore or disable the entire semver job.

## F4. Exact-SHA fuzz smoke

Ensure the fuzz workflow’s target matrix exactly matches every `[[bin]]` target in `fuzz/Cargo.toml`.

For the final candidate SHA:

- run every target;
- record target, duration, run ID, URL, SHA, and result;
- upload crash artifacts on failure;
- do not use a fuzz run from an earlier SHA as release evidence.

Add an automated consistency test or script comparing workflow target names to `fuzz/Cargo.toml` so future targets cannot be omitted silently.

## Workstream F acceptance criteria

- Main CI, RC, and local release validation execute one equivalent blocking contract.
- RC is workspace and CLI complete.
- `cargo audit` and semver checks are included in release validation.
- Strict conformance is blocking everywhere it is release-relevant.
- No release-critical step uses `continue-on-error`.
- The fuzz matrix is automatically checked against `fuzz/Cargo.toml`.
- Every fuzz target runs against the exact candidate SHA.

---

# Workstream G: Evidence ledgers and exact-SHA release closure

## G1. Create status ledgers

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

Each ledger must include:

- disposition: open, partial, superseded, or closed;
- plan baseline SHA;
- implementation SHAs;
- criterion-by-criterion mapping;
- source/test paths;
- exact commands and observed results;
- test counts and ignored-test rationale;
- CI, fuzz, and RC run IDs/URLs;
- artifacts and inspected values;
- known limitations;
- release version containing the work;
- reviewer sign-off or evidence reference.

Historical status files must correct earlier claims where commit messages or changelog statements exceeded the implementation.

## G2. Required Plan 029 matrices

`plans/029-status.md` must contain:

1. v3 extraction path inventory and shared-probe call site;
2. no-legacy-fallback adversarial test matrix;
3. payload emission and channel-bit matrix;
4. `EmbedOutcome` propagation matrix;
5. public resource-entrypoint table;
6. one row per `ResourceLimits` field;
7. detached trust/exit-code matrix;
8. independent fixture provenance table;
9. conflict expectation truth table results;
10. CI/RC command-equivalence table;
11. fuzz-target consistency and run table;
12. exact-SHA release evidence table;
13. unresolved blockers section, empty before release.

## G3. Exact candidate SHA rehearsal

For one final candidate SHA:

1. Start from a clean checkout.
2. Run the complete release-validation contract.
3. Obtain green main CI for that SHA.
4. Obtain green all-target fuzz smoke for that SHA.
5. Obtain green release-candidate workflow for that SHA.
6. Download and inspect conformance, semver, package, tool-version, and commit artifacts.
7. Run `cargo package --workspace` and inspect every `.crate` inventory.
8. Install the CLI from packaged artifacts or a temporary local registry.
9. Run PNG, JPEG, and WebP protect/verify smoke tests.
10. Run detached `keygen -> sign -> verify-manifest` smoke tests.
11. Run attacker substitution, key-material mismatch, untrusted, wrong-key, wrong-image, malformed-manifest, duplicate-key, missing-payload-key, and wrong-payload-key cases.
12. Verify exact exit codes and JSON/human agreement.
13. Record all evidence in `plans/029-status.md`.

## G4. Publication sequencing

Only after the exact-SHA rehearsal is complete:

1. Confirm `0.2.3` has not already been published or reserved. If it has, select the next patch version and update all package/changelog references consistently.
2. Create the immutable tag from the validated SHA.
3. Publish the library crate.
4. Confirm registry and index availability.
5. Publish the CLI crate against the published library.
6. Create the GitHub release from the same tag/SHA.
7. Record crate checksums, package contents, registry references, release URL, and tag SHA.

Do not rebuild or publish from another ref.

## G5. Post-publication verification

From a clean environment:

- install from crates.io;
- verify reported library and CLI versions;
- run PNG/JPEG/WebP protection and verification;
- run trusted and untrusted detached verification;
- run caller-key material mismatch and attacker substitution;
- run correct, missing, and wrong payload-key cases;
- confirm exact human/JSON exit behavior;
- inspect installed package metadata;
- record outputs and checksums.

## Workstream G acceptance criteria

- Plans 021-029 have truthful committed ledgers.
- Every closed criterion cites production code, a test, a command, a run, or an artifact.
- One exact SHA has green CI, fuzz, strict conformance, semver, RC, and package evidence.
- Tag, library crate, CLI crate, and GitHub release all identify that exact SHA/version.
- Post-publication installation and security smoke tests pass.
- The published CLI rejects key-material substitution with exit 3.
- `plans/029-status.md` has no unresolved blocking item.

---

# Required implementation order

1. Workstream A: shared v3 probe and no-fallback behavior.
2. Workstream B: emission context, actual payload sizing, and outcome propagation.
3. Workstream C: operation budget and public-boundary resource closure.
4. Workstream D: detached mismatch semantics, early validation, and CLI adversarial closure.
5. Workstream E: conformance provenance, conflict logic, negative coverage, preservation proof.
6. Workstream F: unified blocking validation and fuzz consistency.
7. Workstream G: status ledgers, exact-SHA RC, publication, and post-publication verification.

Do not begin publication while any criterion in A-F remains unresolved.

---

# Suggested commit boundaries

Use small reviewable commits where practical:

1. `stego: centralize bounded v3 header probe`
2. `stego: remove v3 fixed-window and legacy-fallback paths`
3. `pipeline: propagate embed and metadata outcomes`
4. `pipeline: derive payload claims and capacity from actual emission`
5. `limits: thread operation budget through public byte paths`
6. `detached: classify caller key-material mismatch as integrity failure`
7. `cli: expose trust mode and key-material diagnostics`
8. `conformance: correct provenance and conflict expectations`
9. `conformance: add negative coverage and preservation tests`
10. `ci: unify release validation and fuzz target contract`
11. `plans: add Plans 021-029 evidence ledgers`
12. `release: record exact-SHA candidate evidence`

Do not combine publication with unresolved implementation changes.

---

# Mandatory validation commands

Run and record:

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
```

Feature validation:

```bash
cargo test -p stegoeggo --no-default-features
cargo test -p stegoeggo --no-default-features --features signatures
cargo test -p stegoeggo --no-default-features --features detached-manifest
cargo test -p stegoeggo --no-default-features --features signatures,detached-manifest
cargo test -p stegoeggo --all-features
cargo test -p stegoeggo-cli --all-features
```

Also run and record:

- strict conformance with nonzero source-aware minima;
- every fuzz target;
- all shared-probe and no-legacy-fallback tests;
- payload channel/outcome matrices;
- all public resource-boundary tests;
- detached CLI adversarial tests;
- package-install smoke tests;
- post-publication smoke tests.

Record exact commands and conclusions, not only aggregate counts.

---

# Reviewer checklist

A reviewer must answer yes to every item:

- Does every production v3 path use one shared minimum-header probe?
- Are fixed 36-byte and 48-byte candidate windows absent from initial v3 classification?
- Do tiled paths classify v3 before any v1/v2 decode?
- Does any v3-magic failure prevent legacy fallback?
- Are declared length, extensions, key IDs, auth fields, limits, and arithmetic validated before full extraction?
- Does actual serialized payload length drive all capacity calculations?
- Are payload channel bits based on actual emission rather than configuration defaults?
- Can metadata-only output ever claim a hidden marker?
- Does `EmbedOutcome` reach warning, report, JSON, human, and strict-exit surfaces?
- Is output re-verification no longer the primary success signal?
- Does every public byte-processing entrypoint check limits before expensive work?
- Are seed and origin caps proven by bounded-failure scenarios and observed counters?
- Are resource usage fields populated from production operations?
- Is caller key-material mismatch classified as exit 3?
- Does invalid manifest structure return before hashing or signature evaluation?
- Do CLI JSON and human output show trust mode and key-material match state?
- Does a structurally valid attacker substitution fail at the CLI level?
- Are independent fixture provenance records accurate and reproducible?
- Do all four conflict expectation states behave correctly?
- Does strict coverage fail when required external categories are removed?
- Is preservation tested as a real before/after public operation?
- Do CI, RC, and local validation execute one equivalent blocking contract?
- Is the fuzz workflow automatically synchronized with `fuzz/Cargo.toml`?
- Are Plans 021-029 backed by exact evidence?
- Are CI, fuzz, RC, tag, crates, and release tied to one SHA?
- Do post-publication tests include key-material mismatch and attacker substitution?

---

# Definition of done

Plan 029 is complete only when:

1. Every v3 extraction and verification path uses one bounded minimum-header-first probe.
2. No v3-magic result can enter legacy decoding.
3. Payload claims and capacity calculations reflect actual serialized and emitted evidence.
4. `EmbedOutcome` reaches all public reporting and CLI decision surfaces.
5. Every resource limit is enforced through public production boundaries with observed accounting.
6. Caller-owned key-material mismatch is a structured integrity failure with exit 3 and complete CLI diagnostics.
7. Invalid manifests fail before hashing, signature evaluation, image decoding, or embedded extraction.
8. Independent conformance fixtures have truthful provenance, correct conflict handling, negative coverage, and preservation proof.
9. CI, RC, release validation, semver, audit, conformance, feature tests, and fuzz are blocking and aligned.
10. Plans 021-029 status ledgers are committed and evidence-backed.
11. One exact SHA passes main CI, all-target fuzz, RC, semver, strict conformance, package inspection, and smoke tests.
12. The patch release is published from that SHA only.
13. Post-publication installation and image/detached/security tests pass.
14. `plans/029-status.md` contains no unresolved blocking item.

If any condition is absent, keep Plans 026-029 open and keep `0.2.3` unreleased.