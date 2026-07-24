# Plan 027: Plan 026 Corrective Closure and Release Evidence

## Status

Ready for implementation. This plan is a narrowly scoped corrective pass for defects remaining after the first Plan 026 implementation attempt.

`0.2.3` is currently an unreleased candidate version. Do not create `v0.2.3`, publish either crate, or describe Plans 026–027 as complete until every blocking criterion below is implemented and recorded in `plans/027-status.md`.

## Audited baseline

This plan was written against `main` at:

- `1ed50bf93ea5b8500e92903ceef8df85829b8e71`

Plan 026 implementation commits added useful partial fixes:

- V3 CRC/HMAC fixed-size candidates are tried before v2/v1 candidates.
- DCT warning calculations use the current fixed v3 CRC/HMAC sizes.
- The v3 authentication channel bit reflects whether HMAC is configured.
- Detached CLI parsing uses `DetachedManifest::from_json_with_limits`.
- Detached CLI verification delegates most signature and binding work to the library.
- Embedded references use raw image bytes rather than `DynamicImage` extraction.
- Input and default dimension limits moved ahead of metadata-only processing.
- Root and CLI crate versions were changed to `0.2.3`.

The remaining issues are correctness, integration, and release-evidence defects. They do not justify redesigning the product.

## Objective

Finish the exact closure contract established by Plan 026:

1. Payload v3 extraction is header-driven and honors the declared payload length rather than trying two fixed v3 sizes.
2. Malformed v3, unsupported v3, missing authentication key, failed authentication, valid payload, and no payload are distinguishable.
3. Payload channel flags, capacity results, warnings, and execution reports describe actual emitted evidence.
4. Detached-manifest CLI trust uses the caller-supplied public key correctly, and embedded HMAC references can be verified with an explicit payload key.
5. Manifest parsing rejects ambiguous or malformed key/signature records before signing or verification.
6. Resource limits are enforced through public entrypoints and `ResourceUsage` records production observations rather than direct-test-only setters.
7. Workspace CI, release-candidate validation, independent conformance evidence, status ledgers, and release artifacts close Plans 021–027 truthfully.
8. The corrected code is released once as immutable `0.2.3`, or a later patch number if `0.2.3` becomes unavailable before closure.

## Scope guardrails

This plan must not:

- Add image formats.
- Add steganographic algorithms.
- Add signature algorithms.
- Add C2PA, certificate services, hosted trust stores, registries, or blockchain features.
- Remove v1/v2 readers.
- Redesign the default CLI command hierarchy.
- Perform unrelated performance work.
- Move, recreate, or alter `v0.2.2`.
- Publish current code before closure evidence exists.
- Satisfy acceptance criteria through test-only accessors that bypass production entrypoints.

Split unrelated refactors from corrective commits before review.

## Required implementation order

Perform work in this order:

1. Header-driven v3 extraction and observed embedding outcomes.
2. Detached trust, HMAC-reference verification, and manifest validation.
3. Production resource enforcement and accounting.
4. Workspace CI and independent conformance closure.
5. Status ledgers, exact-SHA release candidate, publication, and post-publication evidence.

Do not begin publication work while Sections 1–4 have unresolved acceptance criteria.

---

## Workstream A: Replace fixed v3 candidate sizes with header-driven extraction

### A1. Define one shared v3 probe contract

Current extraction loops try `V3_CRC_PAYLOAD_BITS`, `V3_HMAC_PAYLOAD_BITS`, v2, and v1. This supports only the current no-extension writer and does not implement the variable-length v3/TLV contract.

Create one internal probe used by LSB, tiled LSB, DCT/F5, tiled DCT/F5, extraction, and verification paths.

The probe must:

1. Extract exactly enough bits to inspect the fixed v3 core header.
2. Determine whether v3 magic is present.
3. Validate before a second extraction:
   - version;
   - header length;
   - total length;
   - authentication algorithm;
   - authentication tag length;
   - key-ID length;
   - extension count/lengths;
   - integer conversions and overflow;
   - `ResourceLimits::max_payload_bytes`.
4. Re-extract exactly `total_length * 8` bits using the same seed, redundancy, tile, coefficient set, and channel.
5. Parse and authenticate the exact payload.
6. Attempt v2/v1 only when v3 magic is absent.
7. Never reinterpret malformed or unsupported v3 bytes as legacy payloads.

A suitable internal model is:

```rust
struct V3Probe {
    total_bytes: usize,
    auth: ObservedPayloadAuth,
    key_id_bytes: usize,
    extension_bytes: usize,
}

enum CandidateOutcome {
    Valid { bytes: Vec<u8>, version: u8 },
    AuthenticationKeyMissing { probe: V3Probe },
    AuthenticationFailed { probe: V3Probe },
    MalformedV3,
    UnsupportedVersion(u8),
    InvalidLegacy(Vec<u8>),
    NotFound,
}
```

Exact names may differ. The required property is semantic separation, not this precise type layout.

### A2. Prevent malformed-v3 legacy fallback

When extracted header bytes begin with v3 magic:

- invalid header length is `MalformedV3`;
- impossible total length is `MalformedV3`;
- excessive total length is a resource/config failure;
- unknown v3 authentication algorithm is unsupported/malformed according to the wire contract;
- truncated exact extraction is malformed/not complete, not v2/v1;
- a wrong HMAC key is `AuthenticationFailed`;
- no HMAC key is `AuthenticationKeyMissing`.

Add a regression test that constructs bytes with valid v3 magic but a malformed header whose first legacy-sized window could otherwise appear parseable. The outcome must remain malformed v3.

### A3. Use actual generated payload bytes for all capacity decisions

Current v3 output is fixed-length, but capacity code must not encode that assumption.

Generate the payload before selecting or warning about capacity and compute:

```text
payload_bits = generated_payload.len() * 8
required_slots = payload_bits * effective_redundancy * channel_spread_factor
```

Apply the exact generated byte length to:

- non-tiled LSB;
- tiled LSB per tile;
- baseline JPEG DCT/F5;
- tiled JPEG DCT/F5 per tile;
- progressive fallback decisions;
- warnings;
- `ExecutionReport` observations.

Legacy constants may remain only in legacy-read paths and frozen compatibility tests.

### A4. Derive every v3 channel flag from the resolved execution plan

Do not hard-code `rights_metadata` or `hidden_marker` to true.

Pass the resolved channel facts into payload generation:

```rust
struct PayloadEmissionContext<'a> {
    protection: &'a ProtectionContext,
    rights_metadata_emitted: bool,
    hidden_marker_selected: bool,
    authentication: PayloadAuthentication,
    tiled: bool,
    progressive_jpeg: bool,
}
```

Required semantics:

- `rights_metadata = true` only when canonical rights metadata is actually emitted.
- `hidden_marker = true` only for a payload that is being embedded.
- `authentication = true` only for cryptographic authentication such as HMAC; CRC32 remains integrity-only.
- tiled/progressive flags reflect the path used, not merely a requested option that degraded or fell back.

Difficult case:

```text
RightsPolicy::Unspecified
rights metadata disabled
hidden marker BestEffort
CRC32 payload

rights_metadata = false
hidden_marker   = true
authentication = false
```

Add tests that parse the bitfield and assert each flag. Checking only magic/version bytes is not sufficient.

### A5. Return observed embedding outcomes

Embedding helpers must not silently return unchanged bytes with only precomputed warnings.

Return a structured outcome from LSB and DCT embedding, for example:

```rust
enum EmbedOutcome<T> {
    Embedded {
        output: T,
        payload_bytes: usize,
        required_capacity: usize,
        available_capacity: usize,
        path: EmbedPath,
    },
    SkippedCapacity {
        output: T,
        payload_bytes: usize,
        required_capacity: usize,
        available_capacity: usize,
        path: EmbedPath,
    },
    UnsupportedProgressive {
        output: T,
    },
}
```

Propagate the observed outcome through the request API so that:

- `process_request_bytes_with_warnings` includes runtime capacity/fallback warnings;
- `process_request_bytes_with_report` derives `stego_attempted` and `stego_succeeded` from the observed outcome;
- strict CLI mode fails when the observed degradation is error-severity for the selected contract;
- reports never claim v3 emission after a capacity skip.

Re-verification may remain defense in depth, but it must not be the only source of runtime outcome information.

### A6. Required tests

Add production-path tests for:

- v3 CRC with a non-empty extension section;
- v3 HMAC with key ID and extension bytes;
- exact-length extraction through PNG, WebP, JPEG, tiled PNG, and tiled JPEG;
- a carrier that fits the actual v3 payload but not the legacy v2 window;
- malformed v3 magic/header cannot fall through to v2/v1;
- unsupported v3 version cannot fall through to v2/v1;
- missing HMAC key distinct from wrong HMAC key;
- wrong HMAC key distinct from corruption where observable;
- parsed CRC/HMAC channel flags;
- rights metadata enabled and disabled flag cases;
- tiny LSB and insufficient DCT carriers produce observed capacity outcomes;
- request warnings include runtime capacity degradation;
- strict CLI exits non-zero for required-channel degradation;
- frozen v1 and v2 fixtures remain readable.

### Workstream A acceptance criteria

- No production v3 extraction path relies on only the two fixed CRC/HMAC lengths.
- All extraction channels call one header-driven v3 probe contract.
- Malformed/unsupported v3 never reaches legacy parsing.
- Missing authentication key and authentication failure are distinct outcomes.
- Capacity is computed from the actual generated payload.
- All channel flags match actual output.
- Runtime embedding outcomes reach warnings, reports, JSON, and strict CLI behavior.
- V1/v2 compatibility tests pass.

---

## Workstream B: Correct detached CLI trust and embedded HMAC verification

### B1. Parse caller public-key files as key plus identity

The current CLI extracts public-key bytes but discards the `key_id:` field and constructs a trusted key with an empty ID. Correct the key-file parser so it returns:

```rust
struct ParsedPublicKeyFile {
    key_id: Vec<u8>,
    verifying_key: VerifyingKey,
}
```

Required behavior:

- Parse the declared key ID when present.
- Validate key-ID encoding and configured maximum length.
- Validate the public key is exactly 32 bytes.
- Derive the key ID only when the format explicitly permits derivation and document the derivation.
- If a declared key ID and derived key ID must match under the chosen format, reject disagreement.
- Construct `TrustPolicy` from the parsed identity, not an empty placeholder.

The external public key must also participate in cryptographic verification when supplied. Do not trust a key ID while verifying only a different public key embedded in the manifest.

Choose one explicit contract:

1. Preferred: supply caller-owned verifying keys to the library verifier, match records by key ID, and verify with those keys.
2. Acceptable: require the manifest public key bytes to equal the caller-supplied trusted public key before treating the signature as trusted.

A manifest cannot substitute arbitrary key bytes under a trusted key ID.

### B2. Add explicit payload-key support

Manifest signature trust and embedded payload HMAC authentication are separate operations.

Add detached verification options such as:

```rust
pub struct DetachedVerificationOptions<'a> {
    pub payload_mac_key: Option<&'a [u8]>,
    pub require_embedded_reference: bool,
}
```

Expose a CLI option with unambiguous semantics, for example:

```text
--key <public-key-file>       caller trust for Ed25519 manifest signatures
--payload-key <hex|@path|->   HMAC key for the embedded payload only
```

The environment-variable form may be supported using the existing key resolver, but do not overload a single value for both Ed25519 trust and HMAC authentication.

### B3. Expand embedded-reference status

Replace the current broad `Malformed`/`Present` result where needed with statuses that preserve operational meaning:

```rust
enum EmbeddedReferenceStatus {
    NotProvided,
    Stripped,
    Malformed,
    UnsupportedVersion,
    VersionMismatch,
    DigestMismatch,
    AuthenticationKeyMissing,
    AuthenticationFailed,
    PresentValid,
}
```

Required semantics:

- HMAC payload + no payload key: `AuthenticationKeyMissing`.
- HMAC payload + wrong payload key: `AuthenticationFailed`.
- CRC payload with valid checksum and matching digest/version: `PresentValid`.
- A valid manifest signature never overrides a failed embedded reference.
- Human and JSON output use the same structured verdict.

### B4. Define overall detached-verification success explicitly

The library result must expose an overall decision derived from:

- manifest structural validity;
- image instance digest;
- format, dimensions, and file size;
- required signature cryptographic validity;
- caller trust;
- declared embedded-reference result.

Do not infer CLI success only from `VerificationReport::summary_status()` if that summary omits an embedded-reference failure.

A suitable model is:

```rust
enum DetachedOverallStatus {
    VerifiedTrusted,
    VerifiedUntrusted,
    InvalidConfiguration,
    BindingFailure,
    SignatureFailure,
    EmbeddedReferenceFailure,
}
```

Map exit codes from this result:

- `0`: verified and trusted, including any declared reference.
- `2`: malformed manifest/key/options or unsupported schema/algorithm.
- `3`: binding, signature, digest, or embedded-reference failure.
- `4`: cryptographically valid evidence exists but caller trust is absent.
- `5`: unexpected internal error.

### B5. End-to-end CLI tests

Add integration tests that invoke the built CLI rather than only calling library functions:

1. `keygen` creates a private/public pair.
2. A manifest is generated for a test image.
3. `sign` signs it.
4. `verify-manifest --key <generated-public-key>` exits `0`.
5. Verification without `--key` exits `4`.
6. Verification with another generated public key exits `3` or `4` according to the documented trust/key-mismatch contract, but never `0`.
7. Replacing manifest public-key bytes while retaining a trusted key ID cannot produce success.
8. Modified image exits `3`.
9. CRC embedded reference succeeds.
10. HMAC embedded reference without `--payload-key` exits `3` and reports missing key.
11. Correct `--payload-key` succeeds.
12. Wrong `--payload-key` exits `3` and reports failed authentication.
13. Human and JSON overall outcomes agree.

### Workstream B acceptance criteria

- Correct generated public-key files produce trusted CLI verification.
- Caller-supplied public key bytes are cryptographically bound to trust evaluation.
- Manifest-controlled key bytes cannot impersonate a caller-trusted key ID.
- Embedded HMAC references can be verified with a separate payload key.
- Missing and wrong payload keys produce different structured statuses.
- Overall detached success includes embedded-reference requirements.
- Exit-code behavior is covered end to end.

---

## Workstream C: Complete manifest validation and atomic signing

### C1. Centralize manifest structural validation

`from_json_with_limits` must call one validator shared by parsing, signing, and verification.

Validate at minimum:

- exact supported schema version;
- maximum manifest bytes;
- maximum record counts;
- supported algorithms and encodings;
- public-key byte decoding and exact length;
- signature decoding and exact length;
- non-empty and bounded key IDs;
- duplicate public-key IDs;
- duplicate signature identity `(algorithm, key_id)`;
- duplicate or conflicting public keys for one key ID;
- signature record with no matching public key, unless the caller-key-only contract explicitly permits it;
- invalid embedded-reference digest syntax and supported digest algorithm;
- canonical claim fields required for verification.

Return structured configuration errors. Do not leave malformed encodings to become generic untrusted results.

### C2. Prevent ambiguous signature append behavior

The CLI `sign` command must reject an existing signature for the same `(algorithm, key_id)` unless an explicit replacement flag is added. Adding a replacement flag is optional; silent duplication is forbidden.

When inserting the public key:

- reuse an identical existing record;
- reject a conflicting key under the same ID;
- do not append duplicates.

### C3. Make signing output atomic

Use the existing atomic-write helper or a shared library equivalent for signed manifests.

Required sequence:

1. Read source bytes.
2. Bounded parse and validate.
3. Construct and validate the updated manifest in memory.
4. Serialize fully.
5. Write a temporary file in the destination directory.
6. Flush and persist/rename atomically.
7. Leave the original file unchanged on any failure before persistence.

Add a test using an invalid/unwritable destination or injected serialization/write failure where practical, proving the original file remains intact.

### Workstream C acceptance criteria

- Duplicate and conflicting identities are rejected before cryptographic work.
- Unsupported algorithms and malformed encodings are configuration errors.
- Signing cannot silently append duplicate signatures or keys.
- Signing uses atomic replacement.
- Parser, signer, verifier, and CLI use the same structural validator.

---

## Workstream D: Enforce resource limits through production entrypoints

### D1. Audit every public byte-processing entrypoint

Create a table in `plans/027-status.md`:

```text
entrypoint | input-size check | dimension check | parser budgets | test
```

Include at minimum:

- request processing;
- request processing with warnings;
- request processing with report;
- legacy byte processing;
- metadata-only processing;
- image verification;
- stego extraction from bytes;
- stego verification from bytes;
- detached verification;
- manifest parsing;
- conformance fixture parsing where production limits are claimed.

Input size must be checked before hashing, decoding, metadata extraction, copying, or unbounded traversal. Dimensions should be checked from container headers before full decode where supported.

### D2. Replace getter-only limit tests with behavior tests

Tests such as asserting `max_tile_extraction_origins() == 1` do not prove enforcement.

For every `ResourceLimits` field, add at least one test that:

- enters through a public or externally reachable production function;
- provides input exceeding that exact limit;
- observes a stable structured failure or bounded iteration result;
- proves work does not continue past the limit where observable.

The status ledger must contain one row per field:

```text
limit field | production enforcement function | public test | observed result
```

No field may be marked closed using only a builder/getter or direct `ResourceUsage` setter test.

### D3. Instrument production usage

Introduce an operation-local budget/usage accumulator passed through the paths that already enforce limits:

```rust
struct OperationBudget<'a> {
    limits: &'a ResourceLimits,
    usage: ResourceUsage,
}
```

Record actual observations when they happen:

- PNG chunks scanned;
- JPEG segments scanned;
- WebP RIFF chunks scanned;
- XMP bytes parsed;
- XML depth/properties visited;
- metadata fields/bytes copied;
- payload bytes probed/extracted/written;
- tile origins attempted;
- verification seeds attempted;
- manifest key/signature records visited.

Thread the resulting usage into `ExecutionReport` and verification reports where exposed.

Do not label output length as peak memory. If exact allocation accounting is unavailable, rename/document the field as an estimate or leave it unavailable.

### D4. Verify limit behavior on malformed containers

Add fail-before-work tests for:

- oversized metadata-only input;
- oversized verification input;
- PNG width/height above defaults without processing `max_dimension`;
- JPEG width/height above defaults;
- WebP width/height above defaults;
- excessive PNG chunks/chunk bytes;
- excessive JPEG segments/segment bytes;
- excessive WebP chunks/chunk bytes;
- excessive XMP bytes;
- excessive XML depth/properties;
- excessive metadata fields/bytes;
- excessive payload length declared in v3 header;
- excessive detached-manifest bytes/records;
- tile-origin cap reached through actual tiled extraction;
- verification-seed cap reached through actual fallback verification.

### Workstream D acceptance criteria

- Every public byte entrypoint applies the applicable default/explicit limits before expensive work.
- Every current limit field has a production enforcement site and public-boundary test.
- Tile and seed caps are tested through real iteration.
- Production processing populates observed usage counters.
- Execution reports do not present placeholder zeros for work that occurred.
- Resource-limit failures are stable in library errors and CLI JSON/exit classification.

---

## Workstream E: Close CI, conformance, ledgers, and release evidence

### E1. Make workspace and CLI validation explicitly blocking

Update CI, release-candidate validation, publication preflight, and `scripts/validate-release.sh` to use an equivalent blocking contract:

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
```

Feature validation must explicitly include:

```bash
cargo test -p stegoeggo --no-default-features
cargo test -p stegoeggo --no-default-features --features signatures
cargo test -p stegoeggo --no-default-features --features detached-manifest
cargo test -p stegoeggo --no-default-features --features signatures,detached-manifest
cargo test -p stegoeggo --all-features
cargo test -p stegoeggo-cli --all-features
```

Keep `cargo fuzz` in its supported nightly job. Enumerate all fuzz targets and run smoke coverage for the exact release SHA.

### E2. Restore independent source-aware conformance minima

Set and enforce:

```text
external_legacy_min       >= 1
external_conflict_min     >= 1
external_preservation_min >= 1
```

Retain external canonical PNG/JPEG/WebP and alternate-prefix requirements.

Each required external fixture must have:

- actual external or independent authoring path;
- tool and version;
- exact reproducible command or documented historical exception;
- existing checked-in config/generator path;
- source-image provenance and license;
- SHA-256 digest;
- exact expected legal/DMI values.

A StegoEggo-generated fixture cannot count as independent by imitation.

Add negative coverage tests proving removal of each independent category fails strict conformance.

### E3. Correct documentation and release notes

Before RC:

- Keep `0.2.3` marked unreleased.
- Correct the `0.2.2` changelog heading to its actual release date.
- Remove claims that a resource closure table or Plan 026 status file exists until committed.
- Document header-driven v3 extraction rather than fixed CRC/HMAC candidate sizes.
- Document detached `--key` versus `--payload-key` semantics.
- Document missing-key and failed-authentication statuses.
- Keep legal/trust language narrow: signature validity proves only that the corresponding key signed the canonical claim bytes.

### E4. Create truthful status ledgers

Create or correct:

- `plans/021-status.md`
- `plans/022-status.md`
- `plans/023-status.md`
- `plans/024-status.md`
- `plans/025-status.md`
- `plans/026-status.md`
- `plans/027-status.md`

Each ledger must include:

- disposition;
- implementation SHAs;
- acceptance-criterion mapping;
- exact commands;
- test counts with ignored-test explanation;
- CI/fuzz/RC run IDs and URLs;
- artifact names and inspected values;
- residual limitations and non-goals;
- release version containing the change.

Commit messages are not evidence. Do not mark a criterion complete without code/test/artifact evidence.

### E5. Exact-SHA release rehearsal

For the final release commit:

1. Start from a clean checkout.
2. Run the blocking validation script.
3. Obtain green main CI for that SHA.
4. Obtain green fuzz smoke for that SHA.
5. Run release-candidate validation for that exact SHA.
6. Inspect:
   - conformance report;
   - external tool versions;
   - package inventories;
   - semver report;
   - commit identity artifact;
   - crate archives.
7. Install the CLI from the packaged artifact or local registry.
8. Execute protect/verify smoke tests for PNG, JPEG, and WebP.
9. Execute `keygen -> sign -> verify-manifest` smoke tests.
10. Execute untrusted, wrong-public-key, wrong-image, missing-payload-key, wrong-payload-key, and malformed-manifest cases and verify exit codes.
11. Confirm publication consumes the successful RC SHA without rebuilding from a different ref.

### E6. Publication and post-publication verification

Only after `plans/027-status.md` contains complete RC evidence:

- Confirm `0.2.3` is still available; otherwise bump consistently to the next patch version.
- Create one immutable tag from the validated SHA.
- Publish the library first.
- Wait for registry/index availability.
- Publish the CLI against the published library.
- Create the GitHub release from the same SHA.
- Verify crate checksums and package contents.
- Install the CLI from crates.io.
- Repeat minimal image and detached-verification smoke tests.
- Record versions, SHAs, tag, run IDs, artifact checksums, and observed command results in `plans/027-status.md`.

### Workstream E acceptance criteria

- CI and RC explicitly test the workspace and CLI.
- All fuzz targets run in the release-SHA smoke job.
- External legacy/conflict/preservation minima are non-zero and independently satisfied.
- Plans 021–027 have truthful status ledgers.
- Green CI, fuzz, and RC evidence exists for one exact SHA.
- The release tag and published crates use that SHA.
- Post-publication installation and smoke tests pass.

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

Also run the feature matrix from E1, strict conformance, all fuzz smoke targets, and the end-to-end CLI cases from B5.

Do not record only aggregate test counts. Record exact commands, conclusions, and failing/ignored-test explanations.

## Reviewer checklist

A reviewer must answer yes to every item:

- Does v3 extraction read the header first and then the exact declared length?
- Can v3 extensions/key IDs be extracted without adding a new fixed-size constant?
- Does malformed or unsupported v3 stop legacy fallback?
- Are missing and wrong HMAC keys distinct?
- Are rights metadata, hidden marker, authentication, tiled, and progressive flags truthful?
- Do runtime capacity skips reach warnings, reports, JSON, and strict-mode exits?
- Does a generated public-key file make `verify-manifest --key` succeed?
- Are caller public-key bytes cryptographically bound to trust?
- Can manifest key bytes spoof a trusted key ID?
- Can an HMAC embedded reference be verified with `--payload-key`?
- Does overall detached status include embedded-reference failure?
- Are duplicate keys/signatures and malformed encodings rejected before signing/verifying?
- Is manifest signing atomic?
- Does every limit field have a production enforcement site and public-boundary test?
- Are usage counters populated by production work?
- Does CI explicitly run CLI tests and workspace clippy/tests?
- Are independent legacy/conflict/preservation minima non-zero?
- Do Plans 021–027 have evidence-backed ledgers?
- Are CI, fuzz, RC, tag, and publication tied to the same SHA?

## Definition of done

Plan 027 is complete only when:

1. Workstreams A–D are implemented and reviewed.
2. All required local commands and feature combinations pass.
3. Strict conformance passes with non-zero independent source-aware minima.
4. Main CI and fuzz smoke are green for the exact release SHA.
5. Release-candidate validation is green for the same SHA and all artifacts are inspected.
6. Plans 021–027 status ledgers are committed and truthful.
7. The immutable patch release is published from the validated SHA.
8. Post-publication installation and image/detached smoke tests pass.
9. `plans/027-status.md` records all evidence and contains no unresolved blocking item.

If any condition is absent, leave Plans 026–027 open and do not describe this line of work as closed.
