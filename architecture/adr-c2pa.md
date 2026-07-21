# ADR: C2PA Integration

## Status

Deferred

## Context

StegoEggo protects images from unauthorized AI use through rights-reservation metadata (XMP, PNG tEXt, JPEG COM) and optional steganographic markers (LSB pixel, DCT coefficient). Release 5 introduces provenance claims and detached signed manifests, enabling tamper-evident protection records that survive metadata stripping.

C2PA (Coalition for Content Provenance and Authenticity) is an emerging standard for content provenance backed by Adobe, Microsoft, the BBC, and others. The `c2pa-rs` Rust library provides parsing and generation of C2PA manifests. C2PA and StegoEggo's detached manifests address overlapping concerns: proving content origin and detecting tampering.

However, the two systems differ in fundamental ways that make integration non-trivial and potentially harmful to StegoEggo's core value proposition.

## Decision

Defer C2PA integration. Do not add `c2pa-rs` as a dependency, do not generate C2PA manifests, and do not add C2PA-specific verification paths in Release 5. Document C2PA as a complementary technology that users can layer on top of StegoEggo's provenance output.

## Consequences

**Positive:**

- Avoids coupling StegoEggo to an unstable external dependency before it reaches 1.0
- Preserves the trust-free design: StegoEggo's detached manifests require no certificates, no trust lists, no online revocation checks
- Keeps binary size and compile time manageable for library consumers
- Lets StegoEggo's provenance story stabilize before adding an external standard
- Avoids sign-maintenance burden of two signing stacks (Ed25519 raw + X.509/COSE)

**Negative:**

- Users who need C2PA compatibility must run a separate tool after StegoEggo processing
- No automatic interop with Adobe Content Authenticity tools or Microsoft C2PA viewers
- Some users may perceive StegoEggo as incomplete without C2PA support

**Mitigations:**

- Document a recommended workflow: run StegoEggo for protection, then `c2pa` CLI for C2PA signing
- Design the detached manifest format so a future bridge can extract StegoEggo claims and embed them in C2PA manifests without re-processing
- Expose raw provenance bytes via a public API so C2PA bridging is a library-level operation, not a pipeline change

## Alternatives Considered

### 1. Ship C2PA generation alongside detached manifests

Generate C2PA manifests with X.509 certificates from a StegoEggo-managed key pair.

**Rejected because:** C2PA requires trust lists and certificate chains rooted in a trust service. StegoEggo's design philosophy is explicitly trust-free — no implicit trust in a third party. A StegoEggo-managed CA would be a new security surface and a single point of failure. Users would need to trust StegoEggo's signing infrastructure, which undermines the "anyone can verify" model.

### 2. Ship C2PA verification only

Parse and verify C2PA manifests produced by other tools, surfacing StegoEggo-compatible fields.

**Rejected because:** Verification-only integration provides limited value without generation. It also creates an asymmetric surface: StegoEggo can read C2PA but cannot produce it, which is confusing. The `c2pa-rs` crate's verification path pulls in trust-list handling and OCSP stapling, adding complexity without proportional benefit.

### 3. Ship a thin C2PA adapter as a separate crate

Publish `stegoeggo-c2pa` as a companion crate that wraps `c2pa-rs` and bridges between StegoEggo's provenance format and C2PA manifests.

**Rejected because (deferred, not eliminated):** This is the most architecturally sound option but is premature before StegoEggo's own provenance format stabilizes. Premature adapter work would lock both sides into format decisions before real-world feedback arrives. This becomes viable once: (a) StegoEggo's detached manifest format is frozen, (b) `c2pa-rs` reaches 1.0, (c) there is concrete user demand for the bridge.

### 4. Replace detached manifests with C2PA entirely

Use C2PA as StegoEggo's provenance layer and drop the custom detached manifest format.

**Rejected because:** C2PA's trust model is incompatible with StegoEggo's requirements. C2PA manifests are signed with X.509 certificates, requiring a trust infrastructure. StegoEggo's use case — legal deterrence against AI training — does not require or benefit from X.509 trust chains. Additionally, C2PA manifests are larger and slower to generate than StegoEggo's compact detached manifests.

## Revisit Criteria

Revisit C2PA integration when **all** of the following are true:

1. **`c2pa-rs` reaches 1.0 stable** — API surface is frozen, semver commitments are in place, maintenance cadence is established
2. **Trust model evolution** — C2PA introduces a trust-free or decentralized trust mode (e.g., key-based verification without centralized CA), or StegoEggo identifies a concrete use case that justifies the trust-list overhead
3. **User demand** — At least 3 distinct users or integrations request C2PA compatibility with specific workflow requirements
4. **Provenance format frozen** — StegoEggo's detached manifest v1 format is stable and has been through at least one release cycle without breaking changes
5. **Binary size budget** — `c2pa-rs` dependency tree adds less than 200KB to the compiled library (currently estimated at 500KB+ with transitive deps)
6. **Security review** — `c2pa-rs` has undergone an independent security audit or has a CVE response track record

If C2PA reaches these milestones before StegoEggo needs it, the adapter crate (`stegoeggo-c2pa`) is the recommended integration path rather than direct pipeline integration.
