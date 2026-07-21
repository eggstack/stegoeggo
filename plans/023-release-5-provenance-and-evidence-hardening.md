# Plan 023: Release 5 — Provenance and Evidence Hardening

## Status

Blocked on completion of Plans 021 and 022.

## Release intent

Release 5 strengthens StegoEggo’s optional provenance channels without changing the primary role of standards-based rights metadata or overstating the legal and forensic meaning of cryptographic evidence.

The current hidden payload is versioned and backward-readable, but it still has limitations:

- Payload v2 uses a compact fixed layout tied to current fields.
- HMAC-SHA256 authentication is truncated to a short tag.
- CRC32-validated payloads are structurally valid but trivially forgeable.
- Shared-secret HMAC proves possession of a secret, not public identity or ownership.
- Verification collapses several concepts into broad statuses and evidence-strength variants.
- There is no detached signed manifest for cases where image-embedded capacity is insufficient or where distribution systems need an auditable sidecar.
- Public APIs named as ISCC may imply stronger standards interoperability than the implementation can guarantee.
- C2PA has not been evaluated through a documented scope, dependency, and compatibility decision.

Release 5 must add an extensible payload format, stronger authentication options, explicit trust semantics, detached signed evidence, and accurate public naming while retaining read compatibility for payload versions 1 and 2.

## Prerequisites

Before implementation begins:

- Plan 021 must be closed with green conformance artifacts.
- Release 4 request/channel architecture must be complete.
- Hidden-marker and authentication selection must be represented through the resolved protection plan.
- Existing v1 and v2 payload fixtures must be frozen and checked in.
- A security review of key handling, signature scope, canonical serialization, and downgrade behavior must be completed before merging cryptographic code.

## Objectives

At completion:

1. Hidden payload v3 is explicitly specified, versioned, bounded, and extensible.
2. HMAC authentication uses a stronger configurable tag length with safe defaults.
3. Optional Ed25519 signatures support public verification without implying trust or ownership automatically.
4. Verification distinguishes parsing, integrity, authentication, signature validity, key identity, trust, and rights claims.
5. Detached signed manifests can bind a file instance, content identifier, rights notice, and provenance claims.
6. Embedded and detached evidence use one canonical claim model and verification vocabulary.
7. Payload v1/v2 read compatibility remains mandatory.
8. Nonstandard or ambiguous ISCC-like API names are replaced through staged deprecation.
9. C2PA integration is decided through an architecture decision record, not added opportunistically.

## Non-goals

Release 5 must not:

- Claim that a valid signature proves copyright ownership.
- Claim that a hidden marker proves model training or infringement.
- Introduce a network trust service, certificate authority, blockchain, or global identity registry.
- Make public-key signing mandatory for metadata-only workflows.
- Remove v1/v2 payload parsing.
- Add DRM or content-access controls.
- Make C2PA a mandatory dependency without an approved ADR and feature boundary.
- Couple detached manifests to one storage provider or URL scheme.

## Threat model and evidence vocabulary

Release 5 must document the difference between:

- **Structural validity:** bytes conform to a known payload or manifest schema.
- **Checksum validity:** corruption was not detected by a non-cryptographic checksum.
- **MAC validity:** a party possessing the shared key produced or modified the authenticated bytes.
- **Signature validity:** a private key corresponding to a public key signed the canonical claim.
- **Key identification:** a key identifier resolved to a specific public key.
- **Trust:** caller policy accepts that key or trust chain for a defined purpose.
- **Claim content:** the signed statement includes a rights notice or provenance assertion.
- **Ownership:** an external legal or organizational fact not established merely by signature validity.

Every public result type and user-facing message must preserve these distinctions.

## Workstream A: Specify hidden payload v3

### A1. Write a normative wire-format specification

Create `architecture/payload-v3.md` describing byte order, field sizes, canonical values, extension rules, authentication coverage, and parsing limits.

The specification must include:

- Magic/domain-separation bytes.
- Payload version.
- Header length.
- Total length.
- Flags.
- Protection channel identifiers.
- Rights-policy identifier or digest, not a second independently mutable policy copy.
- Seed or marker parameters only when required for extraction.
- Content/instance binding fields.
- Key identifier.
- Authentication algorithm identifier.
- Authentication tag/signature length.
- Extension section with bounded TLV or equivalent encoding.

### A2. Use a bounded extensible layout

Prefer a simple deterministic TLV or length-delimited format over an unbounded general-purpose serializer inside stego payloads.

Requirements:

- Unknown noncritical extensions can be skipped safely.
- Unknown critical extensions cause a distinct unsupported result.
- Length arithmetic is checked before allocation.
- Maximum payload size is explicit.
- Duplicate singleton fields are rejected.
- Field ordering is canonical for signing.

### A3. Separate embedded extraction data from signed claims

Only include extraction-essential data and compact claim bindings in the embedded payload. Large legal metadata, certificate chains, or arbitrary JSON belong in detached manifests.

### A4. Domain-separate authentication

Authentication input must include a fixed StegoEggo domain string, payload version, algorithm identifier, and canonical payload bytes. HMAC and signatures must not share an ambiguous byte domain.

### A5. Preserve legacy readers

The parser must:

- Detect v1, v2, and v3.
- Parse each version through a dedicated function.
- Return a common observation model with source version retained.
- Never reinterpret a v1/v2 checksum as cryptographic authentication.

### Acceptance criteria

- `architecture/payload-v3.md` is complete enough for an independent implementation.
- Parser rejects oversized, truncated, duplicate, reordered-invalid, and unknown-critical fields.
- v1/v2 golden fixtures remain readable.
- v3 round-trip and independent parser tests pass.
- Fuzz targets cover v1/v2/v3 dispatch and TLV parsing.

## Workstream B: Strengthen HMAC authentication

### B1. Increase default tag length

The current effective HMAC tag is short. Define a stronger default, preferably at least 128 bits, based on payload-capacity analysis.

If some hidden-marker modes cannot carry the stronger tag:

- Fail validation for authenticated mode, or
- Select a documented larger payload encoding/channel.

Do not silently downgrade tag length.

### B2. Make tag length algorithm-bound

The v3 header must identify the authentication algorithm and tag length. Parser validation must reject unsupported or unsafe combinations.

### B3. Add key identifiers

Allow callers to provide a non-secret key identifier. The identifier must be included in authenticated bytes and returned during verification.

Key identifiers must:

- Be bounded.
- Avoid embedding secret key material.
- Be treated as labels, not proof of identity.

### B4. Harden key handling

- Reject empty and implausibly short HMAC keys.
- Redact keys in debug output.
- Avoid serializing secret keys in request/config structures by default.
- Zeroization may be considered if it can be added without misleading guarantees; document actual memory behavior.
- Use constant-time tag comparison.

### B5. Define downgrade behavior

When a caller requests HMAC authentication:

- Insufficient payload capacity is an error, not a checksum fallback.
- Missing key is an error before processing.
- Verification without the key reports `AuthenticationNotAttempted`, not invalid.
- Verification with the wrong key reports authentication failure distinctly from malformed payload.

### Acceptance criteria

- Default v3 HMAC tag is at least 128 bits or a documented security-equivalent choice.
- No authenticated request silently emits CRC32-only payloads.
- Wrong key, missing key, corrupt tag, and malformed payload have distinct results.
- Keys are absent from debug/serde output and logs.
- Capacity tests cover every format and marker mode.

## Workstream C: Add optional Ed25519 signatures

### C1. Add a feature-gated signing dependency

Use a well-maintained pure-Rust Ed25519 implementation behind an optional feature such as `signatures`.

Requirements:

- No unsafe code in StegoEggo.
- Dependency versions and security posture reviewed.
- Default metadata-only builds do not require signature dependencies.
- docs.rs builds all features successfully.

### C2. Define signing configuration

Add public configuration that separates:

- Signing key material.
- Public verifying key.
- Key identifier.
- Signature placement: embedded binding, detached manifest, or both.

Private keys must not implement accidental serialization or verbose debug output.

### C3. Define canonical signed bytes

Sign a canonical claim structure, not ad hoc container bytes. The signed input must include:

- Domain/version.
- Rights-notice digest.
- Rights policy.
- Content/instance binding.
- Creation/signing timestamp if supplied.
- Key identifier.
- Relevant channel/payload identifiers.

### C4. Define embedded signature strategy

A full Ed25519 signature may exceed capacity for some hidden-marker modes. Choose explicitly:

- Embed the full signature where capacity permits.
- Embed a digest/reference to a detached signed manifest.
- Reject unsupported combinations.

Do not truncate Ed25519 signatures.

### C5. Define verification results

Report separately:

- Signature present.
- Signature structurally valid.
- Signature cryptographically valid.
- Public key supplied/resolved.
- Key identifier matched.
- Key trusted under caller policy.
- Signed claim matches observed file/notice.

### Acceptance criteria

- Valid signatures verify with the expected public key.
- Wrong key, altered claim, altered content binding, malformed signature, and unknown key are distinct.
- Signature validity never sets ownership/trust automatically.
- Feature-disabled builds preserve existing API behavior.
- Test vectors include deterministic known-answer cases.

## Workstream D: Create a unified provenance claim model

### D1. Define `ProvenanceClaim`

Create a canonical, serializable claim type shared by embedded and detached evidence.

Suggested fields:

- Schema version.
- Claim identifier.
- Rights policy.
- Normalized rights-notice digest.
- Content code/digest.
- Instance digest.
- Source file facts.
- Creation/signing time.
- Issuer/key identifier.
- Software identifier/version.
- Optional parent claim reference.
- Optional external statement URI.

### D2. Canonicalize serialization

Choose a deterministic representation for hashing/signing. Standard JSON serialization is insufficient unless canonicalization is explicitly implemented and tested.

Options include:

- A project-defined canonical binary encoding.
- Canonical JSON with a documented RFC/profile.
- Deterministic CBOR with a documented profile.

Select one and freeze it with test vectors.

### D3. Bind notice and file observations

Define whether bindings are:

- Exact file-byte digest.
- Decoded pixel/content digest.
- Standard ISCC code.
- Project-local perceptual fingerprint.

Use distinct field names and algorithm identifiers. Never label a project-local fingerprint as a standard ISCC code.

### D4. Support parent/derivation relationships carefully

Optional parent references may state that a claim was derived from another claim. They do not prove the transformation history without trusted tooling and retained evidence.

### Acceptance criteria

- One claim model serves embedded and detached signatures.
- Canonical bytes are stable across platforms.
- Every digest includes an algorithm identifier.
- Exact-file and perceptual/content bindings are distinguishable.
- Golden serialization and signature vectors are checked in.

## Workstream E: Add detached signed manifests

### E1. Define a standalone manifest schema

Create `architecture/detached-manifest.md` and a public type/API for detached manifests.

The manifest should contain:

- Schema/version.
- Canonical provenance claim.
- One or more signatures/authentication records.
- Public keys or references where appropriate.
- Optional certificate/trust metadata, bounded and feature-gated.
- Optional link to image-embedded manifest digest/reference.

### E2. Provide generation APIs

Support:

- Generate manifest from an input file and resolved protection request.
- Protect image and generate manifest together.
- Generate a manifest without hidden markers.
- Serialize to canonical bytes and a human-readable representation where practical.

### E3. Provide verification APIs

Verification must accept:

- Image bytes.
- Manifest bytes.
- Key resolver/trust policy.

Return a structured result covering schema, signatures, bindings, policy consistency, notice consistency, and trust.

### E4. Define storage neutrality

The core crate must not assume local sidecars, HTTP hosting, object storage, or a specific filename. CLI conveniences may use `.stegoeggo.json` or another documented suffix, but the library consumes bytes/readers.

### E5. Define missing/stripped embedded reference behavior

A detached manifest may still verify against a file when the embedded reference was stripped, but the result must state that only detached evidence remains.

### Acceptance criteria

- Detached manifest generation and verification work without hidden markers.
- Image alteration causes exact-instance binding failure.
- Rights-notice alteration causes claim mismatch.
- Valid signature with untrusted key remains valid-but-untrusted.
- Manifest size and nested data are bounded.
- CLI can create and verify detached manifests.

## Workstream F: Redesign verification result semantics

### F1. Replace broad status collapsing

Introduce structured sub-results, for example:

```rust
pub struct VerificationReport {
    pub rights: RightsVerification,
    pub hidden_marker: HiddenMarkerVerification,
    pub authentication: AuthenticationVerification,
    pub signatures: Vec<SignatureVerification>,
    pub detached_manifest: Option<ManifestVerification>,
    pub bindings: BindingVerification,
    pub trust: TrustEvaluation,
    pub diagnostics: Vec<Diagnostic>,
}
```

### F2. Preserve source attribution

Every observed field or result should identify its source:

- Canonical XMP.
- Legacy metadata.
- Embedded payload version.
- Detached manifest.
- Caller-supplied key/trust policy.

### F3. Define trust policy as caller-owned

The core verifier may provide:

- Trust none.
- Trust exact key set.
- Callback/key resolver.

It must not ship an implicit global trust store.

### F4. Maintain compatibility views

Existing `VerificationStatus`, `EvidenceStrength`, and `NoticeVerification` APIs may remain as deprecated summaries derived from the structured report.

### Acceptance criteria

- Structural, checksum, MAC, signature, key, trust, binding, and ownership-claim states are distinct.
- Legacy summary APIs delegate to structured results.
- No public enum variant claims legal ownership based solely on cryptographic validity.
- JSON serialization is stable and documented.

## Workstream G: Correct ISCC and content-identifier naming

### G1. Audit actual standards compliance

Determine exactly which current values are generated by `iscc-lib` and which values or compositions are project-specific.

The current `Iscc` API combines content, instance, optional metadata, and a project-defined presentation. Document what is standard and what is not.

### G2. Rename ambiguous public APIs

If full interoperability cannot be guaranteed, introduce accurate replacements such as:

- `ContentIdentifiers`.
- `ImageContentCode`.
- `ImageInstanceCode`.
- `StegoEggoFingerprint` for project-local values.

Deprecate ambiguous `compute_iscc*` APIs with migration guidance.

### G3. Preserve standard codes without custom relabeling

Where `iscc-lib` produces standard codes, expose those exact values and metadata. Do not concatenate or transform them into a value described as standard unless the standard permits that composition.

### G4. Bind algorithms explicitly in claims

Detached/embedded claims must include identifier type and version, not an unlabeled string.

### Acceptance criteria

- Public naming accurately distinguishes standard ISCC output from project-local fingerprints/compositions.
- Deprecated APIs remain functional through the compatibility window.
- Standards claims are backed by external test vectors or upstream library documentation.
- Provenance claims identify every digest/code algorithm.

## Workstream H: Decide C2PA integration through an ADR

### H1. Produce `architecture/adr-c2pa.md`

Evaluate:

- Use cases overlapping detached manifests.
- Rust dependency maturity and maintenance.
- Binary size and compile-time impact.
- Format support.
- Signing/key model.
- Trust-list requirements.
- Interoperability testing options.
- Whether StegoEggo should generate, verify, bridge, or merely document C2PA.

### H2. Choose one explicit outcome

- Adopt behind an optional feature in a future milestone.
- Integrate verification only.
- Bridge StegoEggo claims into C2PA assertions.
- Defer with specific conditions for reconsideration.
- Reject as out of scope.

### H3. Keep Release 5 bounded

Unless the ADR demonstrates low-risk, testable integration, Release 5 may complete with a documented deferral. Do not let C2PA destabilize the core provenance work.

### Acceptance criteria

- ADR records alternatives, decision, consequences, and revisit criteria.
- Documentation clearly distinguishes StegoEggo manifests from C2PA.
- No unsupported C2PA interoperability claims are made.

## Workstream I: Key management and CLI behavior

### I1. Add key input methods

Support safe, explicit key sources:

- File path.
- Environment variable where documented.
- Standard input for automation.
- In-memory library API.

Avoid command-line literal private keys because process listings may expose them.

### I2. Add key generation/export commands

If the CLI generates Ed25519 keys:

- Use OS randomness.
- Write private keys with restrictive permissions.
- Require explicit overwrite.
- Clearly label public versus private material.
- Use a documented encoding.

### I3. Add verification trust options

Support exact public-key trust and key-directory/resolver workflows without creating an implicit trust service.

### I4. Redact secrets and sensitive paths

CLI errors, debug logs, and reports must not contain private key bytes or HMAC keys.

### Acceptance criteria

- CLI signing works without private key literals in process arguments.
- Wrong file permissions produce warnings or errors as defined.
- Verification can report valid-but-untrusted.
- Secret-redaction tests cover debug and error paths.

## Workstream J: Testing, fuzzing, and independent validation

### J1. Known-answer cryptographic tests

Check in vectors for:

- HMAC v3.
- Ed25519 signing/verification.
- Canonical claim serialization.
- Manifest digest and signatures.

### J2. Compatibility matrix

Test reading:

- v1 CRC payload.
- v2 CRC payload.
- v2 HMAC payload.
- v3 CRC/structural payload if supported.
- v3 HMAC payload.
- v3 signature/manifest reference payload.

### J3. Negative tests

Cover:

- Truncated payload.
- Unsupported critical extension.
- Duplicate field.
- Wrong HMAC key.
- Modified tag.
- Wrong public key.
- Modified signature.
- Modified notice.
- Modified instance bytes.
- Untrusted key.
- Oversized manifest.
- Deep/nested malicious manifest input.

### J4. Fuzzing

Add fuzz targets for:

- Payload dispatch.
- v3 field parser.
- Canonical claim decoder.
- Detached manifest parser.
- Signature record parser.

### J5. Independent implementation test

Create a small independent parser/verifier fixture or script, preferably outside the production module, that validates published v3 vectors.

### J6. Preserve rights conformance

Run the complete Plan 021 conformance suite. Provenance features must not alter canonical metadata behavior.

### Acceptance criteria

- All known-answer vectors pass.
- Fuzz smoke runs are part of CI or scheduled workflows.
- v1/v2 fixtures remain readable.
- External metadata conformance remains green with and without provenance features.
- Feature combinations compile and test independently.

## Milestone sequence

### Milestone 1: Specification and common claim model

- Payload v3 specification.
- Provenance claim model.
- Canonical serialization.
- Verification vocabulary.
- No production signing yet.

Gate: security/design review and test vectors approved.

### Milestone 2: Payload v3 and stronger HMAC

- v3 parser/writer.
- Stronger HMAC tag.
- Key identifiers.
- Legacy parser compatibility.

Gate: compatibility matrix and capacity tests green.

### Milestone 3: Ed25519 signatures

- Optional feature and key types.
- Signing/verification.
- Structured signature results.
- CLI key handling.

Gate: known-answer and negative cryptographic tests green.

### Milestone 4: Detached manifests

- Manifest schema.
- Generation and verification APIs.
- Embedded reference/binding strategy.
- CLI sidecar workflows.

Gate: cross-platform canonicalization and binding tests green.

### Milestone 5: API naming and C2PA decision

- ISCC/content identifier cleanup.
- Compatibility deprecations.
- C2PA ADR.
- Documentation restructuring.

Gate: API review and migration docs complete.

### Milestone 6: Release validation

- Full feature matrix.
- Fuzzing.
- Conformance preservation.
- Security review.
- CI/artifact evidence.

## Feature and dependency policy

Suggested features:

- `signatures`: Ed25519 and public-key verification.
- `detached-manifest`: detached manifest APIs; may imply `signatures` only if unsigned manifests are not supported.
- `c2pa`: only if approved by ADR.

Default features should remain minimal. HMAC support may remain core because it already exists, but v3 implementation must not force signature dependencies into default builds.

## Release gate

Release 5 is complete only when:

1. Payload v3 has a normative specification and independent vectors.
2. v1 and v2 payloads remain readable.
3. Authenticated mode cannot silently downgrade to CRC32.
4. Default HMAC security is materially stronger and capacity-validated.
5. Optional Ed25519 verification distinguishes validity from trust and ownership.
6. Detached manifests bind claims to image/content observations and verify independently.
7. Verification results preserve structural, integrity, authentication, signature, key, trust, and binding states.
8. ISCC-like naming is accurate and migrated compatibly.
9. A C2PA ADR records an explicit decision.
10. Release 1–4 conformance remains green.
11. `plans/023-status.md` records security decisions, vectors, feature matrix, CI runs, artifacts, and residual risks.

## Handoff requirements

The implementing agent must maintain `plans/023-status.md` with:

- Payload schema revision and specification commit.
- Cryptographic dependency versions and review notes.
- HMAC tag-length decision.
- Key identifier format.
- Canonical serialization choice.
- Test-vector locations.
- v1/v2/v3 compatibility results.
- Feature-matrix results.
- Fuzzing evidence.
- C2PA ADR outcome.
- Main CI run and artifact evidence.
- Explicit statements of what signatures and manifests do not prove.
