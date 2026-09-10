# Plan 091: Transactional JPEG Carrier Semantics and Seed-Hint Correctness

Status: READY FOR IMPLEMENTATION

Parent roadmap: `plans/090-generic-carrier-v1-semantics-correctness-and-reuse-roadmap.md`

Audited baseline: `27bdd3d429d663948021de43d3e6f818fa613319`.

## 1. Objective

Make JPEG carrier operations truthful and transactional at the generic API boundary while preserving StegoEggo's existing best-effort application behavior through explicit parent-owned policy.

This plan fixes the concrete quantization-table seed-hint partial-success defect and introduces an explicit distinction between strict carrier embedding and best-effort redundancy/fallback behavior. It must not silently redefine the existing 0.4 public contract in a patch-compatible release.

## 2. Baseline behavior to preserve or correct

### 2.1 Seed-hint defect

`stegoeggo-stego/src/jpeg_transcoder/stego_f5.rs::embed_seed_in_quantization_tables` constructs 96 bits: 32 bits of `SEED` magic and 64 seed bits. Values below 2 are skipped. The function currently returns `Ok(())` without checking `bit_idx == 96`; `extract_seed_from_quantization_tables` rejects tables yielding fewer than 96 bits.

Required correction: successful public hint embedding must imply that the complete hint was embedded and can be recovered immediately by the paired extractor, subject only to a later transformation of the JPEG.

### 2.2 Payload failure currently mutates the JPEG

`jpeg::embed` currently writes a Q-table seed and emits new JPEG bytes when no payload fits, then reports `embedded == false`. This is application-style degradation hidden inside a generic primitive.

The v1 contract should separate:

- strict payload embedding: either payload-bearing output or a structured failure/non-embedded result with no fallback side effect;
- explicit best-effort behavior: caller opts into redundancy reduction and/or seed-hint degradation;
- explicit `embed_seed_hint`: seed hint only.

The existing 0.4 `embed` behavior is compatibility-sensitive. Do not silently change it unless the versioning decision explicitly permits a breaking change.

### 2.3 Redundancy reduction currently happens implicitly

`jpeg::embed` computes the maximum feasible redundancy and lowers the configured value when necessary. This is useful policy but should be an explicit caller decision in the long-term generic API.

## 3. API transition design

Before editing implementation, record in `plans/091-status.md` the selected compatibility shape. Preferred direction:

```text
strict operation
    requested redundancy must fit
    -> embedded output
    OR explicit InsufficientCapacity

best-effort operation
    may reduce redundancy within documented bounds
    -> report actual redundancy
    optional seed fallback remains an application-layer choice

seed-hint operation
    either complete recoverable 96-bit hint
    OR explicit structured error
```

Names are not prescribed by this plan. Choose names after auditing all public/root call sites, but the semantic split above is mandatory. If an additive 0.x API is used, existing `jpeg::embed` remains a compatibility adapter until the documented v1 removal/change boundary.

Do not encode fallback policy as another boolean. Prefer a small non-exhaustive/options type or distinct methods where illegal combinations are structurally difficult to express.

## 4. Implementation phases

### Phase 0 — call-site and byte-contract inventory

Record:

- every direct `jpeg::embed`, `embed_framed`, `embed_tiled`, `embed_seed_hint`, and Q-table helper call;
- parent application paths that rely on redundancy reduction or seed fallback;
- current tests asserting output mutation on insufficient capacity, if any;
- current JPEG known-answer/container-preservation fixtures;
- whether the same partial-hint defect exists in tiled or progressive fallback paths.

### Phase 1 — transactional seed hints

Add a private capacity/preflight helper for Q-table hint storage. It must count exactly the same eligible positions extraction will read, using checked arithmetic and the same table ordering.

Before modifying any table:

1. parse the JPEG/header;
2. count eligible positions across the supported Q-tables;
3. reject if fewer than 96 positions are available;
4. only then mutate a working header;
5. reassemble;
6. in focused tests, verify `extract_seed_hint(output) == Some(seed)`.

Do not partially mutate caller-owned state and then error. The current APIs operate on immutable input bytes, so emitted output is naturally transactional; preserve that property in any prepared-JPEG API introduced later.

Choose the most semantically accurate existing/new error. If `InsufficientCapacity` is reused, document that its units are hint bits/eligible Q-table positions for this operation; if that would make the generic capacity-unit contract ambiguous, introduce a narrowly scoped error variant instead. Record the choice in the status ledger.

### Phase 2 — strict payload embedding core

Extract one private JPEG embedding core whose inputs explicitly include the selected redundancy and which never applies a seed-only fallback on payload failure. Capacity must be checked before coefficient mutation/encode when possible.

Required invariant:

```text
strict embed success => output contains recoverable payload at requested redundancy
strict embed insufficient capacity => no output carrier mutation is presented as success
```

The payload-success path may still store the seed hint if that is part of the established carrier representation required for extraction/parent compatibility. The plan only prohibits seed-only degradation from being implicit in the strict operation.

### Phase 3 — explicit best-effort compatibility/policy

Build best-effort redundancy reduction as an explicit adapter over the strict core. Parent StegoEggo must be able to preserve its current behavior after Plan 096 moves fallback ownership fully into the parent.

During 0.x:

- retain existing behavior behind existing APIs when semver requires it;
- add deprecation/docs steering callers toward the explicit contract;
- do not remove fields/functions promised through v0.x.

At v1:

- the recommended `embed` contract must be unambiguous;
- any automatic redundancy reduction must be opt-in by a clearly named policy/operation;
- seed-only degradation must not be hidden inside generic payload embedding.

### Phase 4 — framed and tiled parity

Ensure raw, framed, and tiled JPEG APIs share the same policy model. Framing must not accidentally reintroduce implicit reduction/fallback. Tiled mode's fixed redundancy-1 semantics should remain explicit and should still fail/report insufficient per-tile capacity without application fallback.

### Phase 5 — terminology/documentation correction

Correct public docs to state that current DCT carrier capacity is based on eligible AC coefficients with `|coef| >= 2`, not all non-zero AC coefficients.

Document the algorithm as an F5-style/no-zero-coefficient variant and make no conventional-F5 interoperability claim. Remove stale internal wording that implies progressive JPEG DCT embedding support; progressive/multiscan inputs remain unsupported by the generic DCT carrier unless another roadmap adds support.

## 5. Required tests

Add focused tests covering at minimum:

- seed hint with exactly enough eligible Q-table positions succeeds and round-trips;
- seed hint with 95 or fewer eligible positions errors and never returns an unrecoverable successful output;
- malformed JPEG remains a structured malformed-input error;
- strict requested redundancy succeeds unchanged when it fits;
- strict requested redundancy does not silently downgrade when it does not fit;
- explicit best-effort mode selects the expected lower redundancy and reports it;
- strict insufficient-capacity path does not apply a seed-only fallback;
- parent compatibility path still produces the pre-plan best-effort outcome/warnings where required;
- framed/tiled operation semantics match raw operation policy;
- APP/COM/unknown segment preservation remains unchanged on successful supported JPEG embedding;
- progressive/multiscan rejection classification remains stable;
- zero and maximum seed values round-trip where supported.

Keep decode-count tests from Plans 078/080 green.

## 6. Acceptance criteria

- Public `embed_seed_hint` can no longer return success for an incomplete/unrecoverable 96-bit hint.
- A strict JPEG payload operation exists and never silently lowers redundancy or applies seed-only degradation.
- Best-effort reduction is an explicit caller choice.
- Existing 0.4 behavior is preserved through compatibility APIs unless an explicitly documented breaking-version boundary is being executed.
- Parent application behavior needed for existing StegoEggo outputs is regression-tested.
- Capacity terminology matches the actual `|coef| >= 2` carrier set.
- Generic docs distinguish the implementation from conventional/original F5 and do not claim progressive DCT support.
- No JPEG parser/coefficient implementation types become public.
- `cargo test -p stegoeggo-stego`, relevant root integration tests, Clippy, and `./scripts/check.sh` pass.

## 7. Non-goals

No progressive/multiscan/restart JPEG implementation, no new watermark algorithm, no JPEG parser rewrite, no generic authentication layer, no public coefficient API, and no new image format support.
