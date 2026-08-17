# Roadmap 069: Stego API Ergonomics and Application-Adapter Simplification

Status: PARTIAL — post-closure residuals tracked by Plan 075

Audited planning baseline: `main` at `f717fd5c99adf17be2bc94d315c71d18f8c77c3d`

Plan 074 closure note: the four implementation plans (070 application-adapter decomposition, 071 framed carrier convenience API, 072 LSB in-place + bitstream allocation optimization, 073 generic public API hardening) were independently verified against the source in `plans/074-status.md`. A later audit found a missing/chronologically incomplete Plan-073 ledger record, repeated JPEG coefficient decoding during framed extraction, JPEG numeric-validation and error-precedence residuals, biased benchmark evidence, and inaccurate carrier cadence wording. Plans 070-074 implementation remains valid; Plan 075 owns these corrective residuals only.

Predecessor: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md` and its closure through Plan 068.

This roadmap does **not** reopen Roadmap 057. The generic carrier crate split, corrected LSB carrier model, JPEG DCT/F5 carrier, canonical `ResolvedProtectionPlan` execution, private JPEG implementation boundary, and tiled-JPEG single-decode search are treated as established baseline contracts.

Implementation plans:

1. `plans/070-application-stego-adapter-decomposition.md`
2. `plans/071-self-describing-framed-carrier-convenience-api.md`
3. `plans/072-lsb-in-place-and-bitstream-allocation-optimization.md`
4. `plans/073-generic-stego-public-api-hardening.md`
5. `plans/074-stego-ergonomics-evidence-and-documentation-closure.md`

Each implementation plan must create and force-track its own `plans/NNN-status.md` ledger before product-source edits. This planning commit authorizes implementation work but is not completion evidence.

---

## 1. Purpose

Roadmap 057 successfully separated StegoEggo's application semantics from reusable steganographic carrier mechanics. The repository now has the correct broad dependency direction:

```text
StegoEggo rights/policy/provenance application
        |
        | arbitrary application payload bytes + explicit carrier configuration
        v
stegoeggo-stego generic carrier crate
        |
        +-- LSB pixel-domain carrier
        +-- JPEG DCT/F5 encoded-byte carrier
        +-- generic frame format
```

The remaining problems are narrower:

1. `src/protected/steganography.rs` remains a large application adapter containing payload preparation, carrier dispatch, seed discovery, extraction search, verification, legacy compatibility, and tiled orchestration in one source unit. Carrier algorithms are no longer duplicated there, but the application-side cognitive load remains unnecessarily high.
2. `stegoeggo-stego::frame` exists, but normal generic callers still have to manually combine frame encoding, prefix extraction, length recovery, carrier extraction, and frame decoding. The public example that demonstrates framing still knows the framed payload length at extraction time.
3. LSB embedding still has convenience-path allocation costs that are unnecessary for callers that already own a mutable image buffer, and the core bit loop materializes payload bits into intermediate vectors.
4. Public configuration uses panicking validation for invalid redundancy values and the independent carrier crate's documentation can more clearly distinguish raw, framed, in-place, and application-support APIs.

The goal is to make the already-correct architecture easier to understand and easier to consume without creating a framework, changing carrier wire behavior, or weakening compatibility.

Desired final application flow:

```text
ProtectionRequest
    -> ResolvedProtectionPlan
    -> prepare StegoEggo marker bytes
    -> dispatch marker bytes to generic carrier
    -> inject rights metadata
    -> output bytes + warnings/report
```

Desired generic-caller flow:

```text
raw API when caller controls payload length/redundancy
OR
framed convenience API when caller wants self-describing recovery
```

---

## 2. Governing constraints

1. Preserve `#![forbid(unsafe_code)]` in both root and carrier crates.
2. Preserve Roadmap-057 carrier correctness and compatibility. Do not redesign the corrected LSB permutation, F5 coefficient selection, tile-seed derivation, payload-v3, or JPEG support subset.
3. Preserve the public default carrier boundary: JPEG parser, entropy, Huffman, coefficient, transcoder, and F5 implementation types remain private.
4. Preserve `application-support` as a narrow parent-crate-only compatibility/support feature. Do not promote it into the generic stable API.
5. Do not add generalized carrier traits, plugin systems, codec abstraction frameworks, or hypothetical media backends.
6. Do not add new media formats. Scope remains PNG/lossless-WebP pixel-domain behavior and supported JPEG DCT behavior already present.
7. Do not change StegoEggo legal-rights semantics, metadata serialization, authentication semantics, payload-v3 fields, or legacy extraction compatibility except for mechanical movement with equivalent tests.
8. Do not change existing raw generic API behavior merely to make framed convenience methods easier. Raw callers must remain able to embed/extract arbitrary bytes with explicit lengths/configuration.
9. Framed APIs must remain application-neutral. The generic frame must not acquire DMI, rights, provenance, XMP, signing, or StegoEggo payload fields.
10. CRC32 in the generic frame is corruption detection, not adversarial authentication. Documentation must say so.
11. No version bump, crate publication, tag, GitHub Release, release workflow, or CI expansion is authorized by this roadmap.
12. Required repository verification remains `./scripts/check.sh`; focused tests and local Criterion measurements may supplement it but must not become CI gates.
13. Performance changes require measured evidence or clear allocation-count evidence and must preserve deterministic carrier output where the algorithm itself is unchanged.
14. Do not optimize by reducing bounded extraction search coverage or weakening malformed-input checks.
15. Prefer deletion/consolidation over introducing new indirection. A new type or helper must remove a concrete duplication or make a public contract materially clearer.

---

## 3. Current state this roadmap assumes

At the audited baseline:

- workspace members include root `stegoeggo`, `stegoeggo-stego`, `stegoeggo-cli`, and fuzz;
- root depends on `stegoeggo-stego = =0.3.2` with the `application-support` feature;
- `stegoeggo-stego` directly depends only on `image`, `jpeg-encoder`, `crc32fast`, and `thiserror`;
- public carrier modules are `lsb`, `jpeg`, `frame`, `error`, and operation-level result/config types;
- `jpeg_transcoder` and `lsb_internal` remain crate-private;
- root re-exports the generic carrier as `stegoeggo::stego`;
- canonical request execution consumes `ResolvedProtectionPlan` directly;
- tiled JPEG search owns one decoded coefficient context per operation;
- normal LSB public embed returns a cloned `RgbaImage` through `EmbedReport<RgbaImage>`;
- raw generic extraction requires caller-known payload length; JPEG raw extraction additionally requires actual redundancy;
- generic framing already provides a bounded 11-byte header with magic, version, payload length, and CRC32.

If implementation starts from a materially different source state, record the deviation in the Plan 070 status ledger before editing source and adapt the plans conservatively rather than forcing stale line-level assumptions.

---

## 4. Execution order

Execute sequentially:

```text
070 application adapter decomposition
        |
        v
071 self-describing framed convenience API
        |
        v
072 LSB in-place + bitstream allocation optimization
        |
        v
073 public API hardening / documentation contract
        |
        v
074 evidence + architecture/documentation closure
```

Why this order:

- Plan 070 reduces root-side cognitive complexity first without changing carrier behavior.
- Plan 071 adds the missing user-facing convenience layer on top of already-proven raw carrier primitives.
- Plan 072 optimizes LSB internals after public framed semantics are locked so optimization cannot silently change the new convenience behavior.
- Plan 073 handles fallible configuration and public-surface cleanup after the final operation set is known.
- Plan 074 reconciles documentation, examples, tests, and closure evidence only after all behavior is final.

Do not combine all plans into one giant refactor commit. Each plan should be reviewable and revertible independently.

---

## 5. Roadmap-owned outcomes

### 5.1 Root application adapter becomes structurally understandable

`SteganographyProtector` may remain the compatibility/application facade, but one source file must no longer own all of:

- application marker construction;
- carrier embedding dispatch;
- seed discovery;
- extraction/candidate orchestration;
- payload verification/auth classification;
- legacy V1/V2 compatibility behavior.

The decomposition must preserve one application owner and avoid recreating duplicate logic in multiple modules.

### 5.2 Generic framed operations become first-class

A generic caller should be able to write conceptually:

```rust
let report = lsb::embed_framed(&image, payload, &config)?;
let payload = lsb::extract_framed(&report.output, &config)?;
```

and equivalent JPEG operations without retaining the original payload length or the embedding report's actual redundancy.

Raw APIs remain available for callers that want explicit low-level control.

### 5.3 LSB callers can avoid full-image clone when they own the buffer

The public carrier crate should expose an in-place LSB embedding operation for `RgbaImage` that does not allocate a second full image. The existing cloning convenience API remains and should delegate to the same core implementation.

### 5.4 Intermediate bit vectors are removed from the corrected LSB hot path

The corrected LSB embed/extract implementation should read payload bits directly from bytes and accumulate extracted bits directly into output bytes rather than allocating one byte per logical bit.

This is a mechanical optimization; deterministic carrier mapping and output must not change.

### 5.5 Public config misuse does not require panics

Generic library consumers must have a fallible way to validate redundancy/configuration. Existing panicking builders may remain only when required for compatibility and should be documented/deprecated deliberately rather than silently changing behavior.

---

## 6. Verification budget

Required final check:

```bash
./scripts/check.sh
```

Focused commands expected across the implementation plans:

```bash
cargo test -p stegoeggo-stego
cargo test -p stegoeggo-stego --doc
cargo test -p stegoeggo --all-features public_stego
cargo test -p stegoeggo --all-features stego
cargo test -p stegoeggo --all-features tiled
cargo test -p stegoeggo --all-features legacy
cargo test --workspace --exclude stegoeggo-fuzz --all-features
```

Local performance/allocation evidence may use the existing benchmark harness or a narrow temporary measurement harness. Do not create benchmark pass/fail thresholds in CI.

---

## 7. Roadmap acceptance criteria

Roadmap 069 is complete only when all are true:

1. Roadmap 057 remains closed and its carrier contracts are not weakened or relitigated.
2. Root stego application responsibilities are decomposed into clearly named modules while preserving one canonical behavior path.
3. No generic carrier algorithm is copied back into the root application crate.
4. The canonical request path still consumes `ResolvedProtectionPlan` directly.
5. Legacy `ProtectionLevel` / `ProtectionContext` behavior remains an adapter into canonical execution.
6. Generic `lsb::embed_framed` / `extract_framed` equivalents exist and recover payloads without caller-known payload length.
7. Generic JPEG framed extraction recovers payloads without caller-retained `actual_redundancy`, using a bounded deterministic search over the existing valid redundancy domain rather than metadata or StegoEggo-specific state.
8. Framed extraction rejects malformed/oversized frames before attempting unbounded allocation or extraction.
9. Raw generic carrier APIs remain available and behavior-compatible.
10. Public LSB in-place embedding exists and the existing cloning embed API delegates to shared carrier logic.
11. Corrected LSB embed/extract no longer materializes full intermediate payload-bit vectors in the normal path.
12. Deterministic corrected-LSB known-answer output remains unchanged for fixed image/payload/seed/redundancy vectors.
13. Generic users have a documented fallible configuration-validation path for invalid redundancy values.
14. JPEG parser/coefficient/F5 internals remain private and absent from the default public API.
15. `application-support` remains hidden/narrow and is not presented as a general public API.
16. `stegoeggo-stego` README and root generic example accurately demonstrate raw vs framed vs in-place usage.
17. No new generalized trait/session framework, raw codec exposure, new media format, or language binding is introduced.
18. All implementation plans have truthful status ledgers and concrete test/commit evidence.
19. Architecture documentation reflects the final application/carrier boundary.
20. `./scripts/check.sh` passes at final closure.
21. No version bump, publication, tag, release, or CI expansion occurs in this roadmap.

---

## 8. Explicit non-goals

This roadmap does not authorize:

- a new steganography algorithm;
- steganalysis-resistance research;
- visible watermark rendering;
- claims of forensic/adversarial robustness;
- payload-v4;
- changes to HMAC/Ed25519 semantics;
- metadata-standard changes;
- progressive JPEG coefficient embedding;
- arbitrary restart/multi-scan JPEG support;
- raw JPEG coefficient APIs;
- a generic `Carrier` trait;
- session/caching APIs for normal JPEG operations without separate benchmark evidence and approval;
- raw RGB/stride buffer APIs in this pass;
- Python/C/WASM bindings;
- dependency replacement merely for novelty;
- CI/release redesign.

Raw pixel-buffer APIs remain a reasonable future extension if a concrete consumer needs them, but `RgbaImage` plus an in-place path is sufficient for this roadmap and avoids premature abstraction.

---

## 9. Final handoff condition

After Plan 074, a maintainer should be able to answer these questions by reading small, local modules rather than tracing a monolith:

- Where is a StegoEggo marker payload prepared?
- Where is carrier selection performed?
- Where is seed discovery/search performed?
- Where is application verification/authentication classified?
- Where does legacy payload compatibility live?
- How does a generic Rust caller embed raw bytes?
- How does a generic Rust caller embed and later recover a self-describing framed payload?
- How does a performance-sensitive caller avoid cloning an `RgbaImage`?

If any of those still require understanding rights metadata plus JPEG internals plus LSB mechanics simultaneously, Plan 074 must mark the roadmap `PARTIAL` rather than papering over the residual.
