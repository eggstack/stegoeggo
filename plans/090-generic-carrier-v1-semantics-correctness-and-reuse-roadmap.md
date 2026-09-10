# Roadmap 090: Generic Carrier v1 Semantics, Correctness, and Reuse

Status: READY FOR IMPLEMENTATION

Audited baseline: `main` at `27bdd3d429d663948021de43d3e6f818fa613319`.

Predecessors: Roadmaps 057, 069, 076, and 081 are closed baseline work. This roadmap does not reopen the completed carrier-crate split, corrected LSB V2 mapping, framed/tiled carrier APIs, output-domain routing, request/verification convergence, or the Roadmap-081 maintainability work.

## 1. Purpose

`stegoeggo-stego` is now a real standalone arbitrary-payload steganography crate rather than an application-internal implementation detail. The remaining work is narrower but more important for a v1-quality public library: eliminate ambiguous generic-carrier semantics, close two correctness invariants before freezing them, provide reuse-oriented buffer/JPEG APIs without exposing codec internals, and finish the boundary between the generic carrier package and the StegoEggo rights/provenance application.

This roadmap is deliberately not a new-algorithm roadmap. The existing LSB and JPEG DCT carriers should become predictable, auditable building blocks before progressive JPEG support, additional carriers, stronger transform-robust watermarking, or GPU acceleration are considered.

## 2. Research and source audit

The baseline audit found the following actionable issues.

1. JPEG quantization-table seed hints are not transactional. `DctStegoF5::embed_seed_in_quantization_tables` skips quantization values below 2 but does not verify that all 96 bits (`"SEED"` + `u64`) were written before returning success. Extraction correctly requires 96 eligible bits. A public `jpeg::embed_seed_hint` call can therefore return `Ok` for a hint that cannot be recovered.
2. Generic `jpeg::embed` contains application policy. When the requested payload cannot be embedded it still modifies the JPEG by writing a seed hint and returns `EmbedReport { embedded: false, ... }`. It also silently lowers requested redundancy to the largest value that fits. Those policies are reasonable for StegoEggo's best-effort application path but surprising in a generic carrier primitive.
3. `LsbConfig::with_redundancy` and `JpegConfig::with_redundancy` have build-profile-dependent invalid-input behavior: debug builds assert while release builds clamp. The existing `try_*` APIs are correct, but a v1 public configuration model should not have different contracts by compilation profile.
4. Public carrier result vocabulary still contains application-shaped concepts (`UnsupportedProgressive`, `QTableSeedOnly`) even though `EmbedReport` is already sufficient for ordinary generic operations.
5. `stego_permutation_v2` claims a true bijection in parent documentation, but the implementation's 256-step cycle-walk escape falls back to `splitmix64(x) % slot_count` and explicitly states that the fallback is not proven bijective. Current tests establish bounds/distribution and small-domain coverage, not per-seed injectivity over the carrier domain.
6. The generic LSB public API is tied to `image::RgbaImage`; this limits reuse with camera/video buffers, FFI/GUI buffers, ndarray-like storage, and strided images even though the algorithm fundamentally needs only validated RGB/RGBA byte access.
7. JPEG repeated operations are intentionally one-shot in the stable public API. Generic consumers that probe, query several capacities, or perform several extraction attempts must decode coefficients repeatedly. The application already has a private single-decode context, demonstrating the use case without justifying exposure of raw JPEG parser/coefficient types.
8. Parent-level `ProtectionPipeline`/`Protector` remain centered on deprecated `ProtectionLevel`/`ProtectionContext`, while canonical processing is request/plan based. The parent also consumes carrier result types that contain application fallback semantics.
9. Documentation describes JPEG capacity as non-zero AC coefficients while the current F5-style implementation selects coefficients with `|coef| >= 2`; the internal F5 module also contains stale progressive-support wording. The implementation is an F5-inspired no-zero-coefficient variant, not a claim of interoperability with conventional F5 implementations.
10. `SUPPORT.md` describes the `Check` job as required, while GitHub reports `main` as currently unprotected with required-status-check enforcement disabled. CI is green, but the repository claim is stronger than the hosting configuration.

External design references used for this roadmap:

- Rust API Guidelines checklist: https://rust-lang.github.io/api-guidelines/checklist.html — validated arguments, caller-controlled allocation, useful intermediate results that avoid duplicate work, private implementation details, common traits, and future-proof public types.
- Cargo SemVer compatibility guidance: https://doc.rust-lang.org/cargo/reference/semver.html — in pre-1.0 crates, `0.y` is the compatibility boundary; additive public APIs can land compatibly, while removal/renaming/default semantic changes require an explicit breaking-version decision.
- NIST SP 800-38G finite-domain encryption guidance is used only as a mathematical reference for the requirement that a finite-domain permutation be a bijection. This roadmap does **not** propose adopting FF1/FF3 or making cryptographic-security claims for the LSB carrier.

## 3. Architectural end state

### 3.1 Generic operations are strict; application fallback is explicit

Carrier primitives should report what happened to the carrier and should not silently apply a StegoEggo-specific fallback. Exact requested redundancy and best-effort redundancy reduction must be distinguishable caller choices. Seed hints remain a separate explicit JPEG operation. During the 0.x compatibility window, existing ambiguous APIs may remain as deprecated compatibility wrappers; v1 must have one unambiguous documented contract.

### 3.2 Configuration has one invariant in every build profile

Runtime-derived configuration must be represented by validated values. Invalid redundancy, tile geometry, buffer geometry, and search bounds return structured errors instead of asserting in debug and clamping in release. Existing 0.x APIs are retained/deprecated as required by the repository compatibility promise; no build-profile-dependent behavior survives into the v1 surface.

### 3.3 Carrier byte mappings are evidence-backed before freeze

The current LSB V2 permutation must either be proven bijective for the entire accepted carrier domain without relying on the non-bijective fallback, or be explicitly classified as a legacy mapping and superseded by a versioned mapping without silently changing existing bytes. Existing known-answer vectors and previously protected images must remain readable.

### 3.4 Reuse APIs expose capability, not codec internals

A public opaque prepared-JPEG type may retain one coefficient decode for repeated generic operations. JPEG headers, coefficient maps, Huffman state, and F5 implementation objects stay private. LSB gains validated borrowed pixel views so callers can use packed or strided RGB/RGBA buffers without first allocating an `RgbaImage`; existing `RgbaImage` APIs delegate to the same mutation/extraction core.

### 3.5 The parent application owns application semantics

Progressive fallback, seed-only degradation, warning policy, and rights/provenance outcomes belong to `stegoeggo`, not `stegoeggo-stego`. The parent should dogfood strict public carrier APIs where possible and retain hidden `application-support` only for genuine legacy/search compatibility that cannot be expressed generically.

## 4. Child plans

1. `plans/091-transactional-jpeg-carrier-semantics-and-seed-hint-correctness.md`
2. `plans/092-validated-carrier-configuration-and-result-contracts.md`
3. `plans/093-lsb-v2-permutation-invariant-proof-and-compatibility.md`
4. `plans/094-opaque-prepared-jpeg-carrier-api.md`
5. `plans/095-generic-strided-pixel-buffer-lsb-api.md`
6. `plans/096-parent-carrier-boundary-and-v1-api-cleanup.md`
7. `plans/097-carrier-v1-packaging-documentation-and-evidence-closure.md`

Each implementation plan uses its `plans/NNN-status.md` companion as the authoritative execution ledger. Product changes are not complete until the status ledger records tests, compatibility disposition, and final commit evidence.

## 5. Sequencing

Wave A — correctness foundations:

- Plan 091: JPEG seed-hint atomicity plus explicit strict/best-effort JPEG semantics.
- Plan 093: LSB permutation proof/instrumentation and compatibility disposition.

These may run in parallel if Plan 091 owns JPEG files and Plan 093 owns LSB permutation files/tests.

Wave B — public contract:

- Plan 092 after Plan 091 has fixed the JPEG operation semantics. It introduces the validated configuration/result vocabulary needed by later APIs without prematurely removing 0.x compatibility paths.

Wave C — reusable carrier interfaces:

- Plan 094 after 091/092: opaque prepared JPEG API using the settled JPEG contract.
- Plan 095 after 092/093: generic pixel views using the settled config and LSB mapping.

Wave D — parent and release closure:

- Plan 096 after the carrier-facing plans: move fallback/application semantics into the parent and record the v1 disposition of `ProtectionPipeline`, `Protector`, root re-exports, and hidden support.
- Plan 097 last: external-consumer evidence, docs/examples, semver/package audit, release-cadence disposition, and truthful CI/support claims.

## 6. Governing compatibility constraints

1. Preserve `#![forbid(unsafe_code)]` across the workspace.
2. Preserve application payload v1/v2/v3 read compatibility and v3 write bytes unless a separately approved payload-version plan changes them.
3. Preserve existing generic frame v1 bytes.
4. Preserve current LSB V2/JPEG carrier extraction for already-produced images. No known-answer mapping may change silently.
5. Treat documented `stegoeggo-stego` 0.4 stable APIs as compatibility obligations. Additive APIs and correctness fixes may land in 0.4-compatible work; removals, renames, public-field changes, and default semantic changes require an explicit breaking-version boundary.
6. Do not expose JPEG parser headers, raw coefficient maps, Huffman tables, or F5 internals merely to enable reuse.
7. Do not add a generic `Carrier` trait until at least a third materially different carrier demonstrates a stable common abstraction.
8. Do not add authentication to the generic frame. CRC32 remains corruption detection; callers layer HMAC/signatures over payload bytes when needed.
9. Keep tiled searches bounded. No unbounded recovery convenience API.
10. Do not weaken malformed-input/resource-limit behavior to simplify APIs.
11. Parent byte/warning behavior must remain regression-tested while fallback policy moves to the application layer.
12. Every child plan finishes with focused tests and `./scripts/check.sh`.

## 7. Completion criteria

Roadmap 090 is complete only when:

- seed-hint embedding cannot report success without a recoverable complete hint;
- generic JPEG strict and best-effort semantics are explicit and tested, with no implicit fallback in the v1 contract;
- all v1 configuration paths have deterministic validation semantics across debug/release;
- application-only embed outcomes no longer define the generic carrier's recommended surface;
- the LSB V2 permutation claim is either formally/evidence-backed for the accepted domain or explicitly versioned/superseded without breaking compatibility;
- prepared JPEG reuse performs one decode across repeated supported operations while codec types remain private;
- strided RGB/RGBA buffer APIs produce byte-for-byte equivalent LSB mappings to the canonical `RgbaImage` path and never mutate alpha/padding;
- parent StegoEggo explicitly owns progressive/seed-only/best-effort policy and the old trait/pipeline API has a recorded v1 disposition;
- standalone direct-crate consumers compile and run without enabling `application-support`;
- docs describe exact carrier units, F5 variant/interoperability limits, raw-tiled integrity limitations, and authentication boundaries correctly;
- package/MSRV/platform/semver evidence is recorded and repository CI-enforcement claims match actual GitHub configuration;
- `./scripts/check.sh` passes on the final integrated tree.

## 8. Explicitly deferred

Progressive/multiscan/restart-interval JPEG DCT support, new carrier algorithms, transform-robust watermarking, video/audio steganography, GPU acceleration, cryptographic FPE adoption, C2PA integration, a plugin framework, and an encoded-image auto-dispatch facade are separate product decisions. They are not prerequisites for a strong reusable v1 carrier library.
