# Plan 025: Security Integration and Production Closure

## Status

Ready for implementation. Release and publication are blocked until this plan is closed.

## Context

Plans 021–024 have been implemented across a large series of commits. The repository now contains a policy-first request API, a metadata-only path, a versioned conformance report, source-aware external fixture coverage, payload-v3 types and parsers, detached manifests, a structured verification model, resource-limit types, release-candidate and publication workflows, fuzz jobs, support/stability documentation, and extensive test additions.

The implementation is substantial, but a final audit found that several release claims do not match the executable system. The most serious issue is the module presented as Ed25519: it derives a public value by hashing secret bytes and creates a deterministic digest from the public value and message. Verification can reproduce that digest using public data alone, so it is forgeable and is not Ed25519 or a digital-signature scheme. Any embedded or detached provenance claim relying on it is insecure.

Other critical integration gaps remain:

- The image protection pipeline still writes and extracts payload v2, while support and stability documents claim v3 is the default write format and that v3 is readable through the normal image API.
- Detached-manifest parsing and verification do not consistently enforce schema, size, count, algorithm, key encoding, embedded-reference digest, and caller-owned trust semantics.
- The request-based CLI can silently disable rights metadata for an explicitly selected rights policy and defaults to PNG instead of preserving the input format.
- Request execution reports infer hidden-marker success from planned warnings rather than verifying the output channel actually landed.
- `ResourceLimits` defines broad production limits but only the input byte limit is visibly wired into the primary pipeline.
- Release-candidate semver validation is neutralized by `|| true`.
- Publication workflow structure and artifact production contain errors and permissive fallbacks.
- Platform support claims exceed the tested CI matrix.
- Fuzzing does not cover every externally reachable parser, merge/update path, or detached-manifest verifier required by Plan 024.
- Status ledgers for Plans 021–024 are missing, and no inspected green CI/release-candidate evidence is recorded.

This plan is a security-first corrective closure pass. It must correct the implementation and documentation before any release tag or publication. It must not add unrelated features.

## Immediate release hold

Until all blocking criteria in this plan pass:

1. Do not create a release tag.
2. Do not publish either crate.
3. Do not distribute binaries advertised as supporting Ed25519 signatures or payload-v3 image embedding.
4. Treat the `signatures` and `detached-manifest` features as experimental and unsafe for authenticity claims.
5. Add a prominent temporary warning to the relevant module and CLI documentation if corrective work will span more than one implementation commit.

## Objective

At completion, StegoEggo must provide one internally consistent and externally validated production contract:

1. Public-key signatures use a real, reviewed Ed25519 implementation and cannot be forged from public data.
2. The normal image-writing and verification paths write payload v3 by default and read v1, v2, and v3 without silent downgrade.
3. Detached manifests are bounded, canonically signed, algorithm-validated, trust-policy-controlled, and exactly bound to the intended image and embedded payload.
4. Explicit rights policies always produce the configured rights signal or fail configuration validation.
5. Execution reports describe observed output, not merely requested work.
6. Resource limits are enforced at every externally reachable parser and verification boundary before expensive allocation or unbounded iteration.
7. CI, release-candidate validation, publication, support documentation, and package contents describe and enforce the same product.
8. All Plans 021–024 acceptance criteria are closed with status ledgers and inspected CI artifacts.

## Non-goals

This pass must not:

- Add a second signature algorithm.
- Add a network trust service, certificate authority, blockchain, or hosted registry.
- Implement C2PA unless the existing ADR explicitly schedules a narrowly scoped task and all blockers below are already closed.
- Add image formats.
- Add new hidden-marker algorithms.
- Remove v1/v2 payload readers.
- Remove deprecated public APIs outside an approved semver-major release.
- Claim that signatures prove copyright ownership, legal enforceability, model training, or infringement.
- Perform broad performance tuning unrelated to the acceptance criteria.

## Primary affected areas

Expected implementation work is concentrated in:

- `Cargo.toml`
- `Cargo.lock`
- `src/signing/**`
- `src/payload_v3/**`
- `src/protected/steganography.rs`
- `src/protected/resolve.rs`
- `src/detached/**`
- `src/provenance/**`
- `src/verification/**`
- `src/resource_limits.rs`
- `src/lib.rs`
- `src/types.rs`
- `src/error.rs`
- `stegoeggo-cli/src/main.rs`
- `stegoeggo-cli/Cargo.toml`
- `tests/payload_v3_roundtrip.rs`
- `tests/known_answer_vectors.rs`
- `tests/independent_v3_parser.rs`
- `tests/signing_tests.rs`
- `tests/detached_manifest_tests.rs`
- `tests/request_api.rs`
- `tests/verification_report_tests.rs`
- `tests/soak_tests.rs`
- `fuzz/**`
- `.github/workflows/ci.yml`
- `.github/workflows/fuzz.yml`
- `.github/workflows/release-candidate.yml`
- `.github/workflows/release.yml`
- `.github/workflows/publish.yml`
- `scripts/validate-release.sh`
- `SUPPORT.md`
- `STABILITY.md`
- `DEPRECATIONS.md`
- `SECURITY.md`
- `README.md`
- `architecture/payload-v3.md`
- `architecture/detached-manifest.md`
- `architecture/provenance-claim.md`
- `plans/021-status.md`
- `plans/022-status.md`
- `plans/023-status.md`
- `plans/024-status.md`
- `plans/025-status.md`

## Workstream A: Replace the forgeable signature implementation

### A1. Remove the homegrown construction

Delete or quarantine every code path that:

- Derives a purported public key by hashing secret key bytes.
- Computes a purported signature from public key bytes and message bytes alone.
- Verifies a purported signature without a real public-key signature primitive.
- Labels this construction as Ed25519.

Do not preserve insecure write behavior for compatibility. If fixtures were generated using the forgeable construction, classify them as invalid development artifacts and replace them.

### A2. Use a reviewed Ed25519 implementation

Use a maintained Rust implementation such as `ed25519-dalek` or another explicitly reviewed crate with:

- Real RFC 8032 Ed25519 signing and verification.
- Constant-time cryptographic operations provided by the dependency.
- No unsafe code introduced into StegoEggo itself.
- Compatible MSRV or an explicitly approved MSRV change.
- Acceptable license and advisory status.
- Deterministic signatures as defined by Ed25519.

Record the dependency and design decision in `architecture/provenance-claim.md` or a focused ADR.

### A3. Harden key types

Signing-key APIs must:

- Return `Result` rather than panic for invalid key IDs or malformed input.
- Avoid exposing secret key bytes through routine getters unless an explicitly named export method is required.
- Redact secrets from `Debug`, `Display`, errors, CLI output, logs, and serialized structures.
- Zeroize secret material using a reviewed zeroization mechanism where practical.
- Clearly distinguish a 32-byte seed, expanded secret key, public key, and key identifier.
- Define key identifier derivation or caller-supplied identifier validation.

### A4. Add independent cryptographic vectors

Tests must include:

- RFC 8032 Ed25519 test vectors.
- Sign/verify round trips.
- Wrong public key.
- Modified message.
- Modified signature.
- Truncated and oversized signatures.
- Non-canonical or malformed encodings handled according to the dependency contract.
- Verification using an independently generated key/signature fixture outside StegoEggo.
- A regression test proving a public key alone cannot create a valid signature.

### A5. Correct all authenticity language

Documentation and result types must say:

- Signature validity proves only that the corresponding private key signed the canonical claim bytes.
- Trust is caller-defined.
- Ownership and legal authority are external facts.
- Invalid, unknown-key, untrusted-key, and malformed-signature outcomes are distinct.

### Acceptance criteria

- No custom signature arithmetic or digest-based substitute remains.
- RFC 8032 vectors pass.
- The public-key-only forgery regression fails to forge.
- All detached and embedded signature tests use the real implementation.
- `cargo deny`, `cargo audit`, MSRV, and license gates pass with the selected dependency.

## Workstream B: Integrate payload v3 into the real image pipeline

### B1. Define one payload writer contract

The image protection pipeline must use the payload-v3 writer for new hidden markers. Remove the old assumption that `CURRENT_PAYLOAD_VERSION = 2` controls current output.

Define one internal abstraction that returns:

- Canonical v3 payload bytes.
- Authentication mode and tag/signature information.
- Exact payload bit length.
- Required embedding capacity.
- Parse/verification metadata needed by the execution report.

### B2. Preserve v1/v2 read compatibility

The normal image verification path must:

1. Detect and parse v3.
2. Fall back to v2.
3. Fall back to v1.
4. Report the observed payload version.
5. Never reinterpret malformed v3 as valid v2 or v1.
6. Distinguish unsupported version, malformed payload, integrity failure, wrong key, and not found.

Freeze checked-in v1 and v2 fixtures before changing the writer.

### B3. Support v3 across every embedding channel

Integrate v3 payload sizing, embedding, extraction, and verification for:

- PNG/WebP LSB.
- Tiled LSB.
- Baseline JPEG DCT/F5.
- Tiled JPEG DCT/F5.
- Progressive JPEG fallback behavior.

Do not assume the v2 fixed ECC bit length. Capacity calculations must use actual v3 bytes and selected authentication/signature placement.

### B4. Implement no-silent-downgrade behavior

When the requested v3 evidence does not fit:

- Return an explicit structured error or a declared degradation that the caller has opted into.
- Do not silently write v2.
- Do not silently drop authentication.
- Do not silently replace a signature with HMAC or checksum.
- Do not report hidden-marker success when no complete v3 payload was embedded.

### B5. Reconcile HMAC behavior

The v3 path must use the stronger tag length selected by Plan 023. The old v2 8-byte tag remains only for v2 read compatibility.

Verification must distinguish:

- Checksum-only payload.
- HMAC payload with missing key.
- HMAC payload with wrong key.
- Valid HMAC payload.
- Structurally corrupt tag.

### B6. Add independent interoperability tests

Add a small independent v3 encoder/parser test implementation or fixture generator that does not call the production v3 writer/parser. Use it to prove:

- StegoEggo writes bytes matching the normative specification.
- StegoEggo reads independently authored v3 payloads.
- Exact byte order, lengths, flags, extensions, and authentication coverage are correct.

### Acceptance criteria

- New image output containing a hidden marker reports payload version 3 through the normal public verification API.
- v1/v2 checked-in images remain readable.
- No production writer emits v2.
- PNG, JPEG, WebP, tiled, HMAC, checksum, and signature-placement cases have end-to-end tests.
- Support and stability documentation matches executable behavior.

## Workstream C: Correct detached-manifest security and binding

### C1. Define canonical signed bytes

Specify exactly what is signed. Avoid signing a structure that includes its own signatures in a mutable or recursive form.

Prefer a canonical claim/envelope representation with:

- Explicit schema version.
- Deterministic field order and encoding.
- Defined Unicode normalization.
- Defined integer and timestamp representation.
- Domain separation.
- Exclusion of signature records from signed claim bytes unless deliberately covered by an outer envelope.

Test canonicalization independently.

### C2. Add bounded parsing and validation

Replace unbounded direct `serde_json::from_slice` entrypoints with a validated parser that enforces before or during deserialization:

- Maximum manifest bytes.
- Supported schema versions.
- Maximum signatures.
- Maximum public keys.
- Maximum key ID length.
- Maximum strings, certificate-chain entries, and individual certificate size.
- Known algorithms and encodings.
- No duplicate signature/key identities where ambiguity matters.
- Required fields and digest syntax.

Return structured errors.

### C3. Standardize key and signature encodings

Choose one external representation for each binary field, preferably a clearly documented base64 or hexadecimal encoding. The serializer, documentation, CLI, and verifier must agree.

Reject:

- Wrong encoding.
- Wrong length.
- Unknown algorithm.
- Key algorithm/signature algorithm mismatch.
- Duplicate or ambiguous key IDs.

### C4. Make trust exclusively caller-owned

Manifest-provided `trust_metadata` must be treated as an untrusted assertion, not copied into the final trusted result.

The final trust outcome must be derived only from the caller-supplied `TrustPolicy`. Report manifest trust assertions separately as informational metadata.

### C5. Verify embedded references exactly

When `embedded_reference` is present, verification must:

- Extract the embedded payload through the version-aware byte API.
- Verify the declared payload version.
- Compute and compare the declared payload digest.
- Distinguish absent, stripped, malformed, version mismatch, digest mismatch, invalid authentication, and present-valid.

Finding any payload is not sufficient.

### C6. Validate image and claim bindings

Detached verification must report independently:

- Instance digest presence and validity.
- Content/perceptual identifier presence and validity where supported.
- Format, width, height, and file-size claim agreement.
- Embedded-reference agreement.
- Signature validity.
- Trust result.
- Rights-policy claim content.

A valid signature over a claim bound to the wrong image must not produce an overall successful verification.

### Acceptance criteria

- Oversized or over-count manifests fail before unbounded work.
- Caller trust cannot be set by manifest content.
- Embedded payload digest and version are verified.
- Key/signature encodings are consistent across docs, library, and CLI.
- Real Ed25519 detached-manifest vectors pass.
- Wrong-image, wrong-key, untrusted-key, unknown-algorithm, and malformed-manifest cases have dedicated tests.

## Workstream D: Correct request API and CLI policy behavior

### D1. Enforce policy/channel consistency

A non-`Unspecified` rights policy requires `rights_metadata = true`, unless a future explicit detached-only policy mode is designed and documented. For this release:

- Reject a request that specifies a rights policy while disabling rights metadata.
- Reject legal metadata or a non-empty rights notice when rights metadata is disabled.
- Do not merely warn and continue.

### D2. Fix CLI channel derivation

The CLI must enable rights metadata when any of these are present:

- An explicit non-`Unspecified` rights policy.
- A non-empty rights notice.
- Legal metadata.
- A preset requiring metadata.

Do not infer rights metadata only from hidden-marker selection.

### D3. Preserve input format by default

When the user does not provide `--format`, leave `ProcessingOptions.output_format = None`. The resolver must preserve the detected input format.

Do not substitute the global PNG default in the request-based path.

### D4. Report observed execution

`ExecutionReport` must be populated from observed output:

- Re-extract canonical rights metadata and compare it with the resolved policy/notice.
- Re-extract the hidden payload and verify version, integrity, authentication, and requested binding.
- Record whether each requested channel was attempted, emitted, verified, degraded, skipped, or failed.
- Record actual output format and whether transcoding occurred.

Do not infer `stego_succeeded` only from the absence of precomputed warnings.

### D5. Stabilize CLI result and exit semantics

Create one top-level CLI error-to-exit mapping:

- `0`: success.
- `1`: processing or verification failure.
- `2`: command/configuration/input-argument failure.
- `3`: integrity/authentication failure if a distinct code is retained.
- `4`: trust-policy failure if a distinct code is retained.
- `5`: unexpected internal failure.

Exact codes may differ, but they must be documented, tested, and used consistently by all commands and top-level error returns.

### D6. Add machine-readable output

Provide versioned JSON for:

- Resolved-plan dry run.
- Protection execution report.
- Image verification report.
- Detached-manifest verification.

Human output and JSON must derive from the same result structures.

### Acceptance criteria

- Explicit rights policies cannot produce metadata-disabled output.
- No-format CLI operation preserves PNG, JPEG, and WebP inputs.
- Tiny/capacity-insufficient images do not report hidden-marker success.
- CLI exit-code integration tests cover config, I/O, processing, authentication, trust, and internal failures.
- Documented stable CLI commands match the actual parser shape.

## Workstream E: Wire resource budgets through all external entrypoints

### E1. Define the budget ownership model

Add `ResourceLimits` to the canonical request/resolved plan or to an explicit processing/verification options object. Legacy APIs may use defaults or map existing limits into the new model.

### E2. Enforce image and container limits

Before allocation or full traversal, enforce:

- Input bytes.
- Width and height.
- PNG chunk count and individual/aggregate chunk bytes.
- JPEG segment count and individual/aggregate segment bytes.
- WebP RIFF chunk count and individual/aggregate chunk bytes.
- Declared lengths not exceeding remaining buffer.

### E3. Enforce metadata and XML limits

Enforce:

- XMP bytes.
- XML depth.
- XML/RDF property count.
- Metadata field count.
- Per-field bytes.
- Aggregate copied metadata bytes.
- Merge/update operation count where applicable.

### E4. Enforce payload and verification limits

Enforce:

- Payload bytes and extension count/size.
- Detached-manifest bytes and record counts.
- Tile origins.
- Verification seeds.
- Signature/public-key records.
- Certificate-chain records and bytes.

Replace hard-coded limits in the steganography verifier with values from the selected limits.

### E5. Report resource usage

Expose observed `ResourceUsage` through execution and verification reports, including at least:

- Input bytes.
- Dimensions.
- Container elements scanned.
- XMP/metadata bytes.
- Payload bytes.
- Tile origins and seeds tried.
- Manifest/signature/key records.

### E6. Test fail-before-work properties

Add tests proving that oversized declared values fail without large allocations or long loops. Include malicious length fields, deep XML, huge counts, and repeated-update input.

### Acceptance criteria

- Every field in `ResourceLimits` has at least one production enforcement site and one test.
- No externally reachable parser relies solely on documentation for bounds.
- Request, legacy, detached, and verification APIs use consistent default limits.
- Resource-limit failures return structured errors and stable CLI JSON/exit results.

## Workstream F: Repair release-candidate and publication workflows

### F1. Make semver checks actually blocking

Remove `|| true`. Preserve the command exit code while still capturing output, for example with `set -o pipefail` and `tee`.

If no published baseline exists, detect that condition explicitly and record a justified `not_applicable` result rather than suppressing all failures.

### F2. Make workflow YAML valid

Move `environment: crates-io` to the publication job where it is valid and effective. Validate workflow syntax with an appropriate linter or GitHub parser.

### F3. Require approved release-candidate evidence

Publication must not independently re-create a weaker validation path. Use one of:

- A `workflow_run`-based publication flow tied to a successful release-candidate workflow for the exact commit/tag.
- A reusable validation workflow invoked by both release-candidate and publication jobs.
- A manually approved environment that verifies the exact validated SHA and artifact digest.

The published SHA must equal the validated SHA.

### F4. Remove dirty publication

Remove `--allow-dirty` from all publication commands. Ensure crate publication order is correct and the CLI dependency resolves to the just-published library version.

### F5. Fix binary builds and platform artifacts

Use the actual binary name `stegoeggo`. Do not swallow cross-compilation failures.

Build supported binaries on native or correctly provisioned runners:

- Linux x86_64.
- Linux aarch64 if support remains claimed.
- macOS x86_64 and aarch64 if support remains claimed.
- Windows x86_64 if support remains claimed.

Install targets/toolchains explicitly. Each claimed artifact must exist, run `--version`, and be uploaded with an unambiguous name.

### F6. Produce auditable release artifacts

Generate and upload:

- Crate package inventories.
- Unpacked crate test results.
- Binary checksums.
- Dependency inventory or SBOM.
- Conformance report.
- Tool versions.
- Semver report.
- Commit/tag identity.
- Fuzz/soak evidence required by the release gate.

### F7. Align local validation

Update `scripts/validate-release.sh` to match CI exactly. Remove `--allow-dirty`. Add semver, package inventory/allowlist, workspace scope, and feature/platform checks that are practical locally.

### Acceptance criteria

- Deliberately breaking semver causes release-candidate failure.
- Invalid workflow YAML is caught in CI.
- Publication cannot run for an unvalidated SHA.
- No publish or package command uses `--allow-dirty`.
- Every claimed binary artifact is built and smoke-tested without ignored failures.
- A dry-run rehearsal produces all expected artifacts without publishing.

## Workstream G: Expand workspace, feature, platform, and package validation

### G1. Validate the full workspace

Use explicit workspace-scoped commands where appropriate:

```bash
cargo check --workspace --all-features
cargo test --workspace --all-features --no-fail-fast
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

Ensure the CLI and library are both compiled and tested under their intended feature sets.

### G2. Test the real feature matrix

At minimum validate:

- Library default/no-default.
- `async`.
- `signatures`.
- `detached-manifest`.
- Signatures plus detached manifest.
- All features.
- CLI default.
- CLI signatures.

Ensure feature dependencies are explicit and cannot expose detached/signing APIs without their required implementation.

### G3. Match platform claims

Add a platform CI matrix matching `SUPPORT.md`, or narrow the support document to what is actually tested.

Platform jobs must run at least:

- Check/build.
- Unit/integration tests that do not require unavailable external tools.
- CLI smoke tests.
- Package checks where supported.

### G4. Verify packaged contents

Update crate include/exclude policy so required user-facing files are packaged, including whichever of these are part of the supported contract:

- `SUPPORT.md`
- `STABILITY.md`
- `DEPRECATIONS.md`
- Relevant architecture/wire-format specifications.
- Security documentation.
- Examples and migration guides.

Extract `.crate` files and run tests/docs against packaged sources where practical.

### Acceptance criteria

- Workspace and CLI failures cannot be hidden by root-package-only commands.
- Every documented feature combination is continuously validated.
- Platform documentation equals the CI matrix.
- Package inventory has an asserted allowlist and includes required support/spec files.

## Workstream H: Complete fuzzing and adversarial parser coverage

### H1. Add missing fuzz targets

Add dedicated targets for:

- PNG chunk parser and metadata injection/update.
- JPEG marker/segment parser and metadata merge.
- WebP RIFF parser and metadata merge.
- XMP/XML extraction and normalization.
- Metadata conflict/merge policies.
- Repeated/idempotent updates.
- Payload v1/v2/v3 dispatch.
- Detached-manifest bounded parser.
- Detached-manifest verification.
- Provenance claim canonicalization.
- Signature/key record parsing.
- Verification report aggregation.

### H2. Seed corpora

Seed each target with:

- Valid current output.
- Historical v1/v2 fixtures.
- Independently authored v3 fixtures.
- Malformed lengths/counts.
- Conflict and duplicate metadata.
- Detached manifest and real signature vectors.

### H3. Define smoke and depth gates

- PR/main smoke runs should cover every target briefly.
- Scheduled/manual depth runs should cover every target longer.
- Crashes, timeouts, and OOM conditions must upload reproducible artifacts.
- Release-candidate evidence must include the latest green scheduled run or a bounded explicit rehearsal.

### Acceptance criteria

- All externally reachable parser families have direct fuzz coverage.
- Regression tests are added for every discovered crash.
- Fuzz workflow target lists are derived or checked against `fuzz/Cargo.toml` to prevent drift.

## Workstream I: Documentation and stability truth pass

### I1. Correct payload claims

Until v3 is integrated, do not claim v3-only writing. After integration, state precisely:

- Read versions.
- Write version.
- Authentication modes.
- Channel/format limitations.

### I2. Correct signature claims

Do not label any construction Ed25519 until the real implementation and vectors pass. Clearly mark signing/detached APIs experimental until Plan 025 closes.

### I3. Correct CLI documentation

Ensure command names, positional arguments, flags, JSON schemas, exit codes, and examples match the actual Clap parser.

### I4. Correct support claims

Only claim platforms and features that are continuously tested. Explain development-only external tool requirements.

### I5. Clarify stability tiers

Do not mark experimental signing commands stable while the signing module is experimental. Machine-readable schema promises must identify schema names and version fields that actually exist.

### Acceptance criteria

- Automated doctests and CLI help snapshots cover all primary examples.
- No support/stability statement contradicts code or CI.
- Security limitations and legal disclaimers are precise and visible.

## Workstream J: Acceptance ledgers and evidence

### J1. Create missing status files

Add:

- `plans/021-status.md`
- `plans/022-status.md`
- `plans/023-status.md`
- `plans/024-status.md`
- `plans/025-status.md`

Each ledger must list:

- Implementation commit SHAs.
- Criterion-by-criterion status: `PASS`, `FAIL`, or `DEFERRED`.
- Exact test names and commands.
- CI/release-candidate run IDs.
- Artifact names and digests where available.
- Remaining risks.
- Documentation/compatibility decisions.

Do not mark a criterion passed from a commit message alone.

### J2. Reconcile earlier ledgers

Update Plans 019 and 020 only where generated evidence changed. Avoid hard-coded test counts that drift; derive counts from reports where possible.

### J3. Require clean CI evidence

Before closure, record a green main run covering:

- Workspace test.
- Lint.
- MSRV.
- Feature matrix.
- Platform matrix.
- Security/license/advisory checks.
- External integration.
- Strict conformance.
- Fuzz smoke.

### J4. Require release-candidate rehearsal

Run the non-publishing release-candidate workflow for the exact proposed commit and inspect every artifact. Record:

- Validated SHA.
- Semver result.
- Package inventories.
- Conformance `complete = true` and `passed = true`.
- Tool versions.
- Binary smoke tests if part of RC.
- Fuzz/soak evidence.

### Acceptance criteria

- All five status files exist.
- No status file reports closure while an explicit acceptance criterion is pending.
- CI and RC run IDs and artifact names are recorded.
- The validated SHA is the proposed release SHA.

## Required regression tests

Add or preserve tests for at least the following.

### Cryptography

- RFC 8032 vectors.
- Public-key-only forgery is impossible.
- Wrong key/message/signature failures.
- Secret redaction.
- Invalid key ID returns error, not panic.

### Payload integration

- New PNG/JPEG/WebP output writes v3.
- v1/v2/v3 reading.
- Malformed v3 does not downgrade.
- V3 checksum, HMAC, signature reference, and unknown algorithm.
- Tiled and non-tiled capacity behavior.
- Progressive JPEG degradation reporting.

### Detached manifests

- Maximum bytes/counts.
- Unknown schema/algorithm.
- Encoding mismatch.
- Duplicate IDs.
- Caller trust vs manifest assertion.
- Wrong image digest.
- Embedded payload absent/version mismatch/digest mismatch/valid.
- Valid signature but untrusted key.

### Request and CLI

- Explicit policy with metadata disabled is rejected.
- Explicit policy produces externally visible canonical DMI.
- No `--format` preserves input format.
- Capacity failure produces observed failed/degraded channel status.
- JSON schemas round-trip.
- Exit-code matrix.

### Resource limits

- Every `ResourceLimits` field exceeds and fails.
- Declared length beyond buffer.
- Deep XML and property explosion.
- Excessive tile origins/seeds.
- Oversized manifest and payload.

### Release workflows

Add script/unit validation where feasible for:

- Semver failure propagation through `tee`.
- Package allowlist.
- Correct binary name.
- Required artifact existence.
- Valid workflow syntax.
- Validated SHA equality.

## Required validation sequence

The implementing agent must run from a clean checkout.

### Hermetic workspace phase

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check --workspace --all-features
cargo test --workspace --all-features --no-fail-fast
cargo test --workspace --doc
cargo package --workspace
cargo deny check licenses
cargo deny check advisories
cargo audit
```

### Feature phase

Run explicit library and CLI checks for default, no-default, async, signatures, detached-manifest, combined provenance features, and all features.

### Cryptographic phase

Run RFC 8032 vectors, independent signing fixtures, detached-manifest vectors, and public-key forgery regression tests.

### External conformance phase

```bash
cargo test --test external_tools -- --ignored
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict \
  --json conformance-report.json
```

Inspect the report and assert:

- `complete = true`.
- `passed = true`.
- Non-zero fixture count.
- Passing digests and coverage.
- Every required tool discovered and exercised.

### Fuzz phase

Run smoke coverage for every declared fuzz target and record the target list and results.

### Release-candidate phase

Dispatch the non-publishing RC workflow for the exact clean SHA. Inspect all uploaded artifacts before updating the status ledger.

## Final acceptance gate

Plan 025 is complete only when all of the following are true:

1. The fake Ed25519 construction is removed and replaced with a real, independently tested implementation.
2. Public-key-only signature forgery is impossible under the implemented API.
3. Normal image protection writes payload v3 and normal verification reads v1/v2/v3.
4. No silent payload/authentication downgrade exists.
5. Detached manifests are bounded, canonical, caller-trust-controlled, and exactly bound to image and embedded payload.
6. Explicit rights policies cannot silently disable metadata.
7. Execution reports reflect observed output.
8. Every resource-limit field is enforced in production code.
9. Semver, package, and publication gates fail closed.
10. Publication uses the exact successfully validated SHA.
11. Supported platform claims match tested CI.
12. Fuzz coverage includes every external parser and merge/verification family.
13. Plans 021–025 status ledgers exist and contain inspected evidence.
14. A green main CI run and green non-publishing release-candidate run are recorded.
15. No documentation claims v3, Ed25519, platform, CLI, or stability behavior that the executable system does not provide.

After this gate, perform one final audit before tagging. Do not infer release readiness solely from a green unit-test count.