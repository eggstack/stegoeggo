# Plan 064: Stego Architecture Evidence and Documentation Closure

Status: Ready for implementation

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

Depends on: Plans 058-063 complete or explicitly dispositioned.

Audited planning baseline: `main` at `e60b801fda493ddd5e744f5de2a12d7b9489c078`

Authoritative implementation ledger to create before source edits: `plans/064-status.md`

---

## 1. Purpose

Perform the final cross-plan audit for Roadmap 057. This plan does not own new carrier features. It verifies that the prior implementation plans actually produced one simpler architecture, that compatibility claims are supported by focused evidence, and that documentation describes the current source rather than the pre-roadmap pipeline.

A broad passing test suite is necessary but not sufficient. The closure must reconcile source structure, public API, legacy compatibility, performance work, dependency disposition, and architectural documentation against the roadmap acceptance criteria.

If a required criterion is still open, mark this plan and Roadmap 057 `PARTIAL` and name the exact residual. Do not close a required defect by relabeling it as a limitation.

---

## 2. Phase 0 — establish the closure ledger

Before documentation or status edits:

1. Create `plans/064-status.md`.
2. Record the actual baseline SHA at the start of closure.
3. Read `plans/058-status.md` through `plans/063-status.md`.
4. Verify every claimed implementation commit exists in history.
5. Identify every `OPEN`, `PARTIAL`, `BLOCKED`, or evidence-missing row.
6. Do not change prior plan dispositions merely to make the roadmap appear complete.

Create a cross-plan table in `064-status.md` with columns:

```text
Roadmap criterion
Owning plan
Owning status row
Implementation commit(s)
Focused test/evidence
Final disposition
```

---

## 3. Phase 1 — source-boundary audit

Inspect the post-implementation source and prove the intended dependency direction.

Required questions:

1. Where is the current LSB carrier implementation?
2. Where is the legacy LSB compatibility implementation?
3. Where is arbitrary-payload JPEG embedding implemented?
4. Where is StegoEggo payload-v3 construction/parsing implemented?
5. Where is rights-metadata seed discovery implemented?
6. Does any generic carrier module import rights-policy/application types?
7. Does `SteganographyProtector` still contain a duplicate carrier algorithm?
8. Does canonical request execution call `plan_to_context()` or another near-equivalent legacy adapter?
9. Are legacy byte APIs routed into canonical request/plan execution?
10. Did Plan 063 leave the carrier as a module or split it, and does the source match the recorded decision?

Expected final architecture:

```text
StegoEggo application/policy layer
  - ProtectionRequest / ResolvedProtectionPlan
  - rights metadata
  - payload-v1/v2/v3 compatibility
  - seed discovery
  - verification reports
             |
             v
generic carrier facade
  - LSB current scheme
  - optional legacy carrier compatibility helper
  - JPEG DCT F5 encoded-byte facade
  - capacity/results
  - generic frame
             |
             v
low-level image/JPEG machinery
```

If source still has materially duplicated carrier or pipeline logic, closure is not complete even if behavior tests pass.

---

## 4. Phase 2 — compatibility evidence matrix

Run or confirm focused evidence for existing StegoEggo images and APIs.

Required matrix:

### LSB compatibility

```text
legacy PNG fixture -> extracts/verifies
legacy HMAC PNG fixture -> verifies with correct key
legacy HMAC PNG fixture -> fails with wrong key
legacy tiled fixture -> extracts if retained by Plan 058
current-scheme PNG -> round-trips
current-scheme WebP -> round-trips
current-scheme tiled crop -> round-trips
```

### JPEG compatibility

```text
supported baseline JPEG -> arbitrary raw carrier round-trip
supported StegoEggo JPEG -> payload-v3 verifies
supported JPEG unrelated APP/COM segments -> preserved
progressive JPEG -> explicit fallback/unsupported behavior unchanged
restart-bearing/multi-scan unsupported fixtures -> explicit behavior unchanged
Q-table seed hint alone -> not reported as verified payload
```

### Canonical/legacy API compatibility

```text
legacy Standard PNG vs equivalent ProtectionRequest
legacy Standard JPEG vs equivalent ProtectionRequest
legacy Light PNG/JPEG vs equivalent request intent
explicit Unspecified policy
HMAC mode
metadata-only request
format conversion request
```

Where deterministic exact output equality is part of the contract, compare bytes. Otherwise compare resolved semantics, verification, warnings, and container-preservation facts.

---

## 5. Phase 3 — generic public API audit

Treat the public API like an external consumer.

Required checks:

- a test outside private modules imports only `stegoeggo::stego` public symbols;
- arbitrary binary LSB raw round-trip works;
- arbitrary binary JPEG raw round-trip works on supported fixture;
- capacity can be queried before embedding;
- framed LSB extraction recovers length/payload and detects checksum corruption;
- framed JPEG extraction recovers length/payload and detects checksum corruption;
- wrong seed does not produce a valid framed payload;
- generic API does not inject rights metadata;
- generic configs/results do not expose DMI/legal/evidence types;
- unsupported JPEG is distinguishable from insufficient capacity;
- JPEG parser/coefficient data structures remain non-public unless an explicitly approved deviation exists.

If Plan 063 selected `SPLIT`, run the external-consumer check both against the dedicated carrier crate and the root re-export where practical.

---

## 6. Phase 4 — performance/complexity closure

Review Plan 060 evidence rather than inventing a new benchmark program.

Source-level acceptance:

- supported successful JPEG production path does not contain the old repeated full coefficient-clone/re-encode search loop;
- capacity/redundancy selection occurs before final encode;
- tiled LSB production code does not crop/blit a temporary `RgbaImage` for each tile/origin;
- no new generalized trait framework or duplicate config layer was added to obtain these wins.

Record representative before/after benchmark observations if Plan 060 produced them. Do not require a fixed percentage improvement.

If runtime became slower despite structural work reduction, investigate obvious regressions before closure and record the result honestly.

---

## 7. Phase 5 — documentation reconciliation

Update only documentation that is now inaccurate or materially incomplete.

Expected primary files:

```text
README.md
AGENTS.md
architecture/overview.md
architecture/pipeline.md
architecture/protected-steganography.md
architecture/jpeg-transcoder.md
architecture/jpeg-stego-f5.md
architecture/types.md        # only if public generic types belong here
architecture/traits.md       # if Protector's final compatibility role changed
```

Add a new focused architecture document only if the generic carrier API cannot be described clearly in the existing steganography deep dive. If added, prefer one file such as:

```text
architecture/generic-stego-api.md
```

and link it from `architecture/overview.md`.

Required documentation truths:

1. `ProtectionRequest -> ResolvedProtectionPlan` is the canonical application execution path.
2. Legacy `ProtectionContext`/`ProtectionLevel` processing is a compatibility facade if Plan 061 completed as intended.
3. Generic carrier APIs accept arbitrary bytes and do not encode rights semantics.
4. Pixel LSB and JPEG DCT are separate carrier domains.
5. Current LSB embeddings use the corrected carrier scheme; legacy extraction is compatibility-only.
6. Generic raw extraction requires known payload length; generic framed extraction carries its own length/checksum.
7. Generic frame CRC is corruption detection, not authentication.
8. JPEG supported/unsupported subset remains accurately stated.
9. Q-table seed hints are not payload verification.
10. `DynamicImage` APIs do not preserve encoded-file metadata.
11. Plan 063 `SPLIT`/`NO-SPLIT` disposition is accurately reflected.
12. No forensic-survival guarantee is implied.

Remove or correct documentation that still says the monolithic `SteganographyProtector` owns all LSB/DCT mechanics if that is no longer true.

---

## 8. Phase 6 — status-ledger reconciliation

Every Plan 058-064 status ledger must contain:

```text
Plan baseline SHA
Disposition
Acceptance-criterion table
Implementation commits
Commands executed
Tests added/changed
Observed results
Known blockers
Documentation changes
CI evidence
Publication hold
```

Plan 064 should not rewrite implementation history. If a previous plan was partially closed and a later plan repaired it, record the corrective link.

Update Roadmap 057 status only after all roadmap criteria are checked.

Valid final roadmap states:

```text
COMPLETE
PARTIAL — <exact residual plans/items>
BLOCKED — <exact external blocker>
```

Do not use vague “mostly complete.”

---

## 9. Phase 7 — final verification

Required final command:

```bash
./scripts/check.sh
```

Record exact exit status in `plans/064-status.md`.

Also run focused suites sufficient to prove the new boundaries. Exact test names may differ after implementation, but evidence must cover:

```bash
cargo test --workspace --exclude stegoeggo-fuzz --all-features -- lsb
cargo test --workspace --exclude stegoeggo-fuzz --all-features -- jpeg
cargo test --workspace --exclude stegoeggo-fuzz --all-features -- stego
cargo test --workspace --exclude stegoeggo-fuzz --all-features -- compatibility
cargo test --doc -p stegoeggo --all-features
```

If Plan 063 created a carrier crate, include its doctests/tests explicitly if `scripts/check.sh` does not already do so.

No new required CI jobs are needed. If GitHub CI runs for the closure commit, record the exact run/commit evidence when available; do not block local closure solely waiting for optional external evidence if the repository's normal process does not require it.

---

## 10. Final acceptance criteria

Plan 064 and Roadmap 057 may be marked complete only when all of the following are true:

1. Plan 058 proves corrected LSB carrier-space/permutation/capacity behavior.
2. Legacy LSB fixtures still extract after the corrected scheme became default.
3. Plan 059 established application-neutral arbitrary-byte LSB and JPEG carrier modules.
4. Generic carrier modules have no rights-policy/legal/provenance type dependencies.
5. `SteganographyProtector` is an application adapter and does not contain duplicate carrier algorithms.
6. Plan 060 removed the normal supported-JPEG retry/clone/re-encode loop or documented a narrowly test-proven retained fallback.
7. Tiled LSB avoids production per-tile/per-origin full-image-buffer crop allocations.
8. Plan 061 made `ResolvedProtectionPlan` the direct canonical execution state.
9. Canonical request execution no longer reconstructs `ProtectionContext` merely to execute the plan.
10. Legacy byte APIs are adapters into the canonical execution path and pass equivalence tests.
11. Plan 062 exposes a public arbitrary-payload generic LSB API.
12. Plan 062 exposes a public arbitrary-payload generic encoded-JPEG API.
13. Generic capacity preflight agrees with actual embed outcomes.
14. Generic framed operations recover payload length and detect accidental corruption.
15. Generic frame semantics remain independent of payload-v3 rights/provenance semantics.
16. JPEG internals remain private behind the public facade.
17. Generic APIs do not silently discover seeds from rights metadata.
18. Existing rights metadata, payload-v3, HMAC, verification, and container correctness remain passing.
19. Plan 063 records an evidence-based `SPLIT` or `NO-SPLIT` disposition.
20. If split, dependency direction is one-way and root public re-exports remain compatible; if no-split, no placeholder crate exists.
21. README/rustdoc/architecture docs describe the final architecture accurately.
22. All Plans 058-064 have truthful status ledgers.
23. `./scripts/check.sh` passes on the final closure state.
24. Required CI remains the existing simple check workflow.
25. No version bump, tag, crates.io publication, GitHub Release, or automated release workflow occurs.

---

## 11. Explicit closure non-goals

Do not add work during closure for:

- new carrier algorithms;
- new image formats;
- cryptographic encryption;
- new signatures/authentication protocols;
- Python/C/WASM bindings;
- steganalysis benchmarks;
- generalized plugin traits;
- progressive JPEG DCT support;
- arbitrary transformation robustness;
- release/publication automation.

Any such idea belongs in a separately approved future roadmap.

---

## 12. Final handoff statement

When complete, `plans/064-status.md` should end with a concise statement identifying:

- final Roadmap 057 disposition;
- final implementation head SHA;
- Plan 063 split disposition;
- exact `scripts/check.sh` result;
- any non-blocking accepted limitations that were already outside roadmap scope;
- explicit statement that no publication action occurred.

If even one roadmap acceptance criterion remains genuinely open, name it and leave the roadmap `PARTIAL`.