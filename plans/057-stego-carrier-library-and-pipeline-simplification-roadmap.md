# Roadmap 057: Stego Carrier Library and Pipeline Simplification

Status: COMPLETE — final tiled-JPEG runtime residual closed by Plan 068

Audited planning baseline: `main` at `e60b801fda493ddd5e744f5de2a12d7b9489c078`

Plan 067 correction note: the post-Plan-066 audit found a tiled-JPEG
candidate-identity regression and evidence inaccuracies. Those residuals are
closed by `plans/067-status.md` after the required focused, CI-equivalent, and
staged pre-release checks passed.

Plan 068 correction note: a later source audit found that the new exact-key
tiled-JPEG operation re-decoded the JPEG for every staged candidate request.
That runtime/resource residual is closed by `plans/068-status.md` after the
operation-local single-decode search context and required verification passed.

This roadmap follows the completed container-correctness work through Plan 056. It does not reopen the PLUS/XMP/WebP/JPEG-container closure except where a regression test is required to prove that stego refactoring preserves those contracts.

Implementation plans:

1. `plans/058-lsb-carrier-correctness-and-compatibility.md`
2. `plans/059-generic-stego-carrier-core-extraction.md`
3. `plans/060-stego-runtime-and-allocation-optimization.md`
4. `plans/061-canonical-plan-pipeline-convergence.md`
5. `plans/062-public-generic-stego-api-and-framing.md`
6. `plans/063-carrier-crate-split-decision.md`
7. `plans/064-stego-architecture-evidence-and-documentation-closure.md`

Each implementation plan must create its own `plans/NNN-status.md` ledger before product-source edits. Do not create completion claims from this planning commit alone.

---

## 1. Purpose

The current repository contains two useful but partially entangled systems:

1. the StegoEggo application protocol for rights-reservation metadata, payload-v3 provenance fields, seed discovery, verification, and legal-notice reporting; and
2. generic steganographic carrier machinery that can embed arbitrary byte payloads in RGB LSB carriers or JPEG DCT coefficients.

The carrier machinery is already substantially reusable internally, but the public surface routes embedding through `ProtectionContext` and `SteganographyProtector`. The canonical request API resolves `ProtectionRequest` into `ResolvedProtectionPlan`, then reconstructs a legacy `ProtectionContext` before execution. This leaves application policy, payload framing, carrier mechanics, format routing, and metadata injection more coupled than necessary.

The goals of this roadmap are therefore:

- correct the LSB carrier model before exposing it as a reusable API;
- preserve extraction compatibility for images produced by the existing LSB algorithm;
- extract LSB and JPEG F5 operations into application-neutral carrier modules;
- make the StegoEggo rights/provenance layer consume those carrier modules rather than own their mechanics;
- simplify successful JPEG stego execution and tiled pixel processing without weakening fail-closed behavior;
- make `ResolvedProtectionPlan` the canonical execution configuration instead of translating it back into `ProtectionContext`;
- expose a deliberately small generic Rust stego API for arbitrary payload bytes;
- add a minimal self-describing generic frame for callers that do not know payload length at extraction time;
- decide separately, from measured dependency and maintenance cost, whether the carrier API warrants a dedicated workspace crate;
- leave CI, release cadence, image-format scope, rights semantics, cryptographic provenance, and legal claims outside this line of work.

The desired final dependency direction is:

```text
CLI / legacy compatibility APIs
             |
             v
ProtectionRequest -> ResolvedProtectionPlan
             |
             +------> rights metadata writer
             |
             +------> StegoEggo payload-v3 adapter
                            |
                            v
                   generic stego carriers
                   +-------------------+
                   | LSB | JPEG DCT F5 |
                   +-------------------+
```

The generic carrier layer must not depend on `RightsPolicy`, `DmiValue`, `LegalMetadata`, `ProtectionLevel`, `EvidenceProfile`, or `StegoPayload`.

---

## 2. Governing constraints

1. Preserve `#![forbid(unsafe_code)]`.
2. Preserve PNG, JPEG, and WebP as the complete image-format scope for this roadmap.
3. Do not add AVIF, HEIF, TIFF, GIF, video, audio, or generalized media steganography.
4. Do not implement a new JPEG codec. Retain the currently proven sequential-Huffman DCT subset and its explicit unsupported fallbacks.
5. Do not change legal-rights semantics, PLUS serialization, XMP ownership rules, or detached-manifest trust semantics except where an adapter must be moved without behavior change.
6. Payload v1/v2/v3 compatibility is retained. This roadmap does not authorize a payload-v4 redesign merely to expose generic carriers.
7. Existing images produced by the current LSB mapping must remain extractable by StegoEggo after a corrected carrier scheme is introduced.
8. New generic carrier APIs must accept caller-provided arbitrary bytes. They must not manufacture rights-specific payloads.
9. Seed discovery from XMP, JPEG Q-tables, or StegoEggo fallback bits belongs to the application adapter, not the raw generic API.
10. The public generic API must keep JPEG internals opaque. `JpegHeader`, entropy tables, and the coefficient-map representation remain implementation details unless a later separately approved plan exposes them.
11. Do not add a generalized trait hierarchy merely to make LSB and JPEG look identical. Pixel-domain and encoded-JPEG-domain carriers may have different concrete APIs behind one small facade.
12. Required CI remains `./scripts/check.sh` and the existing single CI job. No matrix, release workflow, benchmark gate, or external-tool gate is authorized.
13. Release remains manual and out of scope. Do not bump versions, publish crates, tag releases, or create GitHub Releases.
14. Performance changes must preserve byte/container correctness and verification behavior; speed is not a justification for removing fail-closed validation of unsupported/malformed input.
15. The workspace-crate split in Plan 063 is a decision gate, not a predetermined outcome. `NO-SPLIT` is a valid successful disposition when measurements show insufficient benefit.

---

## 3. Current architectural defects this roadmap owns

### 3.1 LSB carrier-space mismatch

The current LSB implementation computes required capacity as though three RGB channels are independent carrier slots, while the permutation operates over pixel indices and chooses the channel separately from the payload-bit index. Logical carrier selections can therefore wrap/reuse the same pixel/channel even after the capacity check succeeds. The affine mapping is also only automatically a permutation when its multiplier is coprime to the modulus; an odd multiplier is insufficient for arbitrary non-power-of-two image sizes.

Plan 058 owns correction and compatibility versioning. Do not silently replace the legacy mapping and strand existing images.

### 3.2 Application-specific wrapper around generic payload carriers

The low-level LSB and F5 routines already accept `&[u8]`, but they are buried inside `SteganographyProtector`, whose surrounding methods generate and parse StegoEggo payloads and discover seeds from rights metadata.

Plan 059 establishes an application-neutral carrier boundary and makes `SteganographyProtector` an adapter rather than the carrier implementation owner.

### 3.3 Defensive JPEG retry loops and avoidable pixel allocations

Successful JPEG embedding can clone coefficient maps and perform repeated encode/decode/extract attempts. Tiled LSB currently allocates/copies subimages for crop and blit operations. These are optimization targets only after carrier correctness and module boundaries are explicit.

Plan 060 owns these changes and must prove equivalent output semantics before deleting defensive work.

### 3.4 Canonical request plan translated back to legacy context

The request path resolves into `ResolvedProtectionPlan`, then uses `plan_to_context()` and older `ProtectionPipeline` methods. This duplicates configuration and maintains multiple execution paths for Light/Standard/request processing.

Plan 061 makes the resolved plan the direct execution input and moves legacy APIs to compatibility adapters into the canonical path.

### 3.5 No small public arbitrary-payload API

`SteganographyProtector` is public, but the useful embedding primitives are crate-private and parameterized by rights-oriented context. Generic users cannot directly ask for “embed these bytes in this RGBA image/JPEG and later extract them with this seed.”

Plan 062 exposes a narrow `stego` API and a minimal generic frame without exposing rights-specific payload types.

---

## 4. Execution order and dependency graph

Execute sequentially:

```text
058 LSB correctness + legacy extraction compatibility
  |
  v
059 internal generic carrier extraction
  |
  v
060 carrier runtime/allocation optimization
  |
  v
061 canonical plan pipeline convergence
  |
  v
062 public generic API + generic frame
  |
  v
063 measured crate-split decision
  |
  v
064 evidence/documentation closure
```

Plans may not be reordered as follows:

- Do not publish the generic LSB API before Plan 058 fixes/version-controls the carrier mapping.
- Do not optimize the JPEG retry loop before Plan 059 separates carrier mechanics from StegoEggo payload construction.
- Do not delete legacy pipeline code before Plan 061 proves compatibility adapters against canonical request execution.
- Do not split a crate before Plan 062 establishes the actual public API boundary.
- Do not mark this roadmap complete before Plan 064 reconciles architecture documentation and status ledgers.

---

## 5. Required compatibility model

Carrier compatibility and payload compatibility are separate concerns.

For LSB:

```text
legacy carrier scheme -> existing StegoEggo payload v1/v2/v3
current carrier scheme -> existing StegoEggo payload v3
```

The corrected carrier scheme must not require changing payload-v3 simply to locate/extract the payload. StegoEggo extraction may probe the current scheme first and then the legacy scheme under bounded resource limits.

For generic callers:

- raw carrier extraction requires the caller to supply the expected payload length and seed;
- framed extraction may infer payload length from the generic frame;
- generic framing is not StegoEggo payload-v3 and does not encode rights policy;
- raw generic APIs do not search metadata for a seed;
- the JPEG Q-table seed channel, if exposed, is an explicit optional hint API rather than an implicit requirement.

---

## 6. Verification budget

Required final automatic verification remains:

```bash
./scripts/check.sh
```

Focused commands may include:

```bash
cargo test --workspace --exclude stegoeggo-fuzz --all-features -- lsb
cargo test --workspace --exclude stegoeggo-fuzz --all-features -- dct
cargo test --workspace --exclude stegoeggo-fuzz --all-features -- stego
cargo test --workspace --exclude stegoeggo-fuzz --all-features -- pipeline
cargo test --workspace --exclude stegoeggo-fuzz --all-features -- compatibility
cargo test --doc -p stegoeggo --all-features
```

Criterion or focused benchmark commands may be used to record before/after observations in Plan 060 and Plan 063. They are measurements, not CI gates.

Do not add routine long fuzzing, exhaustive feature powersets, repeated clean builds, operating-system matrices, or benchmark thresholds to required CI.

---

## 7. Roadmap acceptance criteria

Roadmap 057 is complete only when all of the following are true:

1. New LSB embeddings use a collision-free carrier-slot mapping for every supported positive image dimension within resource limits.
2. LSB capacity reporting uses the same carrier model as embedding and extraction, including replication/redundancy requirements.
3. Existing StegoEggo LSB fixtures produced by the legacy mapping remain extractable after the new scheme lands.
4. Generic LSB carrier operations accept arbitrary payload bytes and contain no dependency on rights-policy/application payload types.
5. Generic JPEG carrier operations accept arbitrary payload bytes and preserve the supported JPEG container contracts already established in Plans 048/053/055.
6. `SteganographyProtector` delegates carrier mechanics rather than implementing its own duplicate LSB/F5 algorithms.
7. Successful supported JPEG embedding performs one normal coefficient embedding/encode path without the current multi-attempt clone/re-encode loop, unless a focused fallback is proven necessary and documented.
8. Tiled LSB embedding/extraction no longer requires allocating full temporary tile images for every tile/origin.
9. `ResolvedProtectionPlan` is consumed directly by the canonical request execution path; request execution no longer reconstructs a legacy `ProtectionContext` merely to run stego/metadata stages.
10. Legacy `ProtectionLevel`/`ProtectionContext` entry points still behave compatibly but are adapters into the canonical execution path rather than independent Light/Standard implementations.
11. A public generic stego API exists for raw arbitrary-payload LSB and JPEG operations with explicit seed/configuration and capacity reporting.
12. A minimal generic framed API can recover payload length and validate accidental corruption without importing StegoEggo rights semantics.
13. The generic public API does not expose JPEG parser/coefficient internals as stable public data structures.
14. Generic carrier operations never silently discover a seed from XMP/legal metadata; StegoEggo-specific seed discovery remains in the application adapter.
15. Plan 063 records a measured `SPLIT` or `NO-SPLIT` decision. A crate is added only if the dependency/maintenance evidence meets that plan's criteria.
16. Existing rights metadata, payload-v3, verification, progressive/unsupported-JPEG fallback, and container-preservation tests remain passing.
17. All Plans 058-064 have truthful status ledgers with implementation commits and focused evidence.
18. Architecture documentation describes the final carrier/application/pipeline boundary rather than the superseded mixed architecture.
19. `./scripts/check.sh` passes at final closure.
20. No release, version bump, tag, publication, or CI expansion occurs as part of this roadmap.

---

## 8. Explicit non-goals

Do not use this roadmap to:

- claim forensic or adversarial watermark robustness beyond current best-effort semantics;
- add image transformation survival guarantees;
- implement steganalysis resistance research beyond the existing algorithms;
- replace the `image` crate;
- support progressive JPEG coefficient embedding;
- support restart-bearing or arbitrary multi-scan JPEG coefficient embedding;
- redesign payload-v3 rights/provenance semantics;
- add new authentication/signature algorithms;
- redesign detached manifests;
- add metadata standards;
- add network APIs or services;
- add Python, C, WASM, or other language bindings;
- add plugin systems or carrier traits for hypothetical future media types;
- publish a new crate merely because a module can technically be separated.

---

## 9. Final handoff condition

Plan 064 must leave the repository with a single understandable architecture:

```text
policy/request resolution
        -> application payload + metadata intent
        -> generic stego carrier
        -> encoded image
```

A future maintainer should be able to use the low-level stego library without learning PLUS/DMI/legal-notice semantics, and should be able to modify the StegoEggo rights application without reimplementing LSB/F5 carrier mechanics.

If any compatibility, carrier correctness, or canonical-pipeline criterion remains open, Plan 064 must mark the roadmap `PARTIAL` and identify the exact residual. It must not close the roadmap by documenting a required defect as an accepted limitation.
