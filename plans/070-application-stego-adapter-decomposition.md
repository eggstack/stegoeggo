# Plan 070: Application Stego Adapter Decomposition

Status: Ready for implementation

Roadmap: `plans/069-stego-api-ergonomics-and-adapter-simplification-roadmap.md`

Audited implementation baseline: `main` at `f717fd5c99adf17be2bc94d315c71d18f8c77c3d`

Authoritative implementation ledger to create before product edits: `plans/070-status.md`

---

## 1. Purpose

The carrier split is complete, but the root application adapter remains concentrated in `src/protected/steganography.rs` (approximately 190 KiB at the planning baseline). That file no longer owns raw carrier algorithms; it delegates LSB and JPEG mechanics to `stegoeggo-stego`. It still mixes several distinct application responsibilities:

- constructing current StegoEggo hidden-marker payloads from `ResolvedProtectionPlan` and legacy context;
- dispatching those bytes to LSB/JPEG/tiled carrier operations;
- discovering seeds from application evidence channels;
- orchestrating non-tiled and tiled extraction candidates;
- parsing/classifying current and legacy payloads;
- verifying CRC/HMAC/signature/application semantics;
- maintaining legacy V1/V2 and legacy API compatibility.

This plan is a behavior-preserving source decomposition. Its primary success metric is responsibility separation and removal of cross-concern helper tangles, not LOC reduction for its own sake.

Do **not** redesign payload formats or carrier behavior in this plan.

---

## 2. Required end state

Keep `SteganographyProtector` as the root application facade if that remains the least disruptive public/internal contract, but organize its implementation under a directory module with explicit responsibility boundaries.

Preferred shape:

```text
src/protected/steganography/
    mod.rs          facade, shared application types, high-level entry points
    marker.rs       prepare/build current application marker bytes
    embed.rs        carrier selection and embedding dispatch
    extract.rs      seed discovery and extraction/candidate orchestration
    verify.rs       payload integrity/auth/application verification helpers
    legacy.rs       V1/V2 and legacy API compatibility adapters
```

Exact names may differ when the source shows a better boundary, but the final structure must satisfy the responsibility criteria below.

A safe migration sequence is:

1. mechanically move `src/protected/steganography.rs` to `src/protected/steganography/mod.rs` with no logic changes;
2. verify;
3. extract one concern at a time into sibling files;
4. verify after each extraction;
5. delete superseded helpers instead of leaving forwarding chains with no compatibility purpose.

Rust visibility should remain as narrow as practical (`pub(super)` / private). Do not make internal application helpers public merely because they move to sibling modules.

---

## 3. Frozen contracts

The following are immutable for Plan 070:

1. Current payload-v3 wire bytes for equivalent deterministic inputs.
2. Legacy V1/V2 extraction behavior and fixture compatibility.
3. CRC/HMAC/Ed25519 verification semantics.
4. Seed discovery order and classification semantics.
5. Fallback-seed behavior and its feature gating.
6. JPEG baseline/progressive/unsupported fallback behavior.
7. JPEG Q-table seed-hint behavior.
8. Non-tiled and tiled LSB mapping and search coverage.
9. Tiled-JPEG exact-candidate identity and single-decode search-context behavior from Plans 067/068.
10. `max_origins`, nearby grid bounds, and redundancy bounds.
11. Warning/degradation reporting.
12. `ResolvedProtectionPlan` as the canonical execution input.
13. Root -> `stegoeggo-stego` operation-level boundary.
14. No direct root imports of JPEG parser/coefficient/F5 internals.

If moving code reveals an existing bug, record it in the ledger. Fix it only if the correction is required to make the decomposition safe and can be proven narrowly; otherwise create a later issue/plan rather than expanding this pass.

---

# Phase 0 — create the status ledger and source inventory

Create `plans/070-status.md` and force-track it before source edits.

Record:

- starting HEAD and working-tree status;
- current workspace versions and root carrier dependency;
- current size/line count of `src/protected/steganography.rs`;
- all `impl SteganographyProtector` blocks and their rough responsibilities;
- all free functions/types/constants defined in the module;
- imports from `stegoeggo_stego` and `application_support`;
- direct callers in `src/lib.rs`, verification modules, tests, and CLI if any.

Start at least these status rows `OPEN`:

```text
R01 mechanical file-to-directory module move preserves behavior
R02 current marker construction has one clear owner
R03 carrier embedding dispatch has one clear owner
R04 seed discovery/extraction orchestration has one clear owner
R05 application verification/auth classification has one clear owner
R06 legacy V1/V2 and legacy API compatibility have one clear owner
R07 no carrier algorithms copied into root
R08 no JPEG parser/coefficient/F5 internals cross boundary
R09 canonical ResolvedProtectionPlan execution preserved
R10 tiled JPEG candidate identity/single-decode behavior preserved
R11 deterministic payload/carrier compatibility tests pass
R12 focused root stego tests pass
R13 full workspace checks pass
R14 architecture docs deferred truthfully to Plan 074
```

---

# Phase 1 — mechanical module conversion

Convert the file module to a directory module without changing logic:

```text
src/protected/steganography.rs
    -> src/protected/steganography/mod.rs
```

Do not combine this move with helper renames or algorithm edits. Keeping a clean mechanical commit makes later review much easier.

Verify that `src/protected/mod.rs` continues to use the same module name and that public/re-export behavior does not change.

Acceptance:

- diff is a path move plus only import/path corrections required by Rust module resolution;
- focused stego tests and `cargo check` pass;
- no observable API or wire behavior changes.

---

# Phase 2 — extract marker preparation

Move application marker creation into `marker.rs`.

This module should own operations that answer:

> Given already-resolved StegoEggo application configuration, what bytes should be placed in the hidden carrier?

Examples of appropriate ownership:

- building V3 payload fields from `ResolvedProtectionPlan`;
- deterministic timestamp/content-hash/key-id/auth field preparation already part of current behavior;
- invoking the existing `payload_v3` writer;
- compatibility construction required by old entry points when it is still truly application payload preparation.

It must **not** own:

- pixel permutation;
- DCT coefficient operations;
- image format routing beyond what is needed to determine application marker semantics;
- XMP parsing/metadata injection;
- generic frame encoding;
- rights metadata serialization.

Prefer a small internal value such as:

```rust
pub(super) struct PreparedMarker {
    pub(super) bytes: Vec<u8>,
    pub(super) seed: u64,
    pub(super) requested_redundancy: usize,
}
```

only if it replaces repeated tuples/parameter chains. Do not add the type if raw `Vec<u8>` plus existing plan fields is clearer.

Acceptance:

- current marker construction has one source owner;
- embed/extract modules do not recreate payload-v3 fields;
- deterministic V3 known-answer/round-trip tests remain unchanged.

---

# Phase 3 — extract carrier embedding dispatch

Move application-level embedding orchestration to `embed.rs`.

This module may decide among existing operations such as:

```text
LSB
LSB tiled
JPEG DCT/F5
JPEG tiled
Q-table seed-only fallback
metadata-only degradation path as signaled to caller
```

It must delegate mechanics to `stegoeggo-stego` rather than implement carrier math.

Preferred call shape:

```text
ResolvedProtectionPlan + PreparedMarker + carrier input
    -> select existing carrier operation
    -> map carrier StegoError/EmbedReport/EmbedOutcome into application outcome/warnings
```

Consolidate duplicated plan-vs-legacy embedding wrappers where the legacy wrapper can simply prepare equivalent inputs and call the canonical helper.

Do not introduce a generic `Carrier` trait. JPEG and LSB consume different domains and explicit dispatch is acceptable.

Acceptance:

- all root carrier mutations route through this module or a clearly documented single facade path;
- no duplicated LSB/F5 loops appear;
- supported JPEG preserving path and progressive fallback tests pass;
- tiled embedding tests pass.

---

# Phase 4 — extract seed discovery and extraction orchestration

Move extraction/search orchestration to `extract.rs`.

This module should own the application question:

> Given an image and StegoEggo's evidence/search policy, which candidate carrier payload should be attempted and in what bounded order?

Appropriate responsibilities:

- application seed discovery from Q-table hints / metadata-derived state / allowed fallback seeds;
- current-vs-legacy carrier probing order;
- non-tiled vs tiled fallback order;
- construction/reuse of `TiledJpegSearch` from `application-support`;
- candidate continuation semantics;
- obtaining raw candidate payload bytes for parsing/verification.

It must not expose or manipulate private JPEG coefficient structures.

Preserve exact Plan-067/068 behavior:

```text
one TiledJpegSearch per root tiled search
prefix candidate key == header key == full/legacy extraction key
```

Acceptance:

- no verification method independently reimplements candidate enumeration;
- tiled evaluator remains shared if currently shared;
- wrong-first/later-valid regressions pass;
- decode-count regressions remain one full decode per tiled-JPEG operation.

---

# Phase 5 — extract verification/authentication classification

Move application payload validation to `verify.rs`.

This module should own:

- parsing/classifying extracted application payload bytes;
- CRC/HMAC/signature verification already present;
- constant-time MAC comparison behavior;
- conversion into existing verification status/result classifications;
- current-vs-legacy payload validation decisions.

Avoid turning `verify.rs` into a second search orchestrator. It should consume candidate bytes/context from `extract.rs` rather than discover carrier candidates itself.

Preserve the distinction between:

```text
seed evidence found
payload bytes extracted
payload structurally valid
payload integrity/auth valid
application verification result
```

Acceptance:

- failure classifications remain test-equivalent;
- wrong key still fails as before;
- CRC-only mode remains explicitly non-cryptographic;
- constant-time comparison remains in the authenticated path.

---

# Phase 6 — isolate legacy compatibility

Move compatibility-only logic to `legacy.rs` where feasible.

Legacy includes two different concepts; keep them explicit:

1. legacy public/context adapters that translate old API inputs into canonical request/plan semantics;
2. legacy hidden-payload V1/V2 extraction/verification.

Do not move canonical current behavior into `legacy.rs` merely because an old entry point calls it.

If legacy adapters already live more appropriately in `src/lib.rs` or another module, do not relocate them gratuitously. The acceptance criterion is that legacy-only stego logic is recognizable and does not clutter current marker/embed paths.

Acceptance:

- V1/V2 fixtures still extract;
- legacy API equivalence tests pass;
- current V3 path does not depend on legacy payload constructors except explicit compatibility fallback.

---

# Phase 7 — consolidation audit

After extraction, audit for forwarding/helper chains created by the move.

Delete helpers that only call another helper with the same arguments unless they preserve a real public/compatibility boundary.

Search specifically for:

- duplicate `*_from_plan` and context variants where one can be an adapter;
- duplicated payload length constants/branching;
- repeated seed/redundancy normalization;
- repeated carrier error mapping;
- duplicated tiled candidate evaluation.

Do **not** chase unrelated generic cleanup.

Structural acceptance criteria:

1. No single module owns marker construction + carrier embedding + extraction search + verification simultaneously.
2. `mod.rs` reads as a facade/orchestrator, not a renamed monolith containing nearly all old implementation.
3. Sibling modules use narrow visibility and no new stable public API is created.
4. Root still imports only operation-level carrier APIs.
5. No algorithm/wire-format changes are required to explain the refactor.

---

# Phase 8 — verification

Minimum focused verification:

```bash
cargo test -p stegoeggo --all-features stego
cargo test -p stegoeggo --all-features tiled_jpeg
cargo test -p stegoeggo --all-features legacy
cargo test -p stegoeggo --all-features payload
cargo test -p stegoeggo --test public_stego_api --all-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

Also run source-boundary searches showing that root does not reference carrier-private types.

Record exact commands/results in `plans/070-status.md`.

---

## Completion criteria

Plan 070 is COMPLETE only when:

- all R01-R14 rows are CLOSED with evidence;
- decomposition is behavior-preserving;
- no product-facing API was expanded unnecessarily;
- all existing carrier and payload compatibility remains intact;
- `./scripts/check.sh` passes;
- no version/release/CI change occurred.

Do not update Roadmap 069 to COMPLETE; Plans 071-074 remain.