# Plan 065 Status Ledger

Status: CLOSED — core Plan 065 implementation and Plan 066 follow-up are complete.

The historical rows below are retained as evidence of the Plan 065 work. Their
remaining residuals are not silently relabeled as complete; Plan 066 is the
final closure owner for the public-boundary, semantic, release-check, and
planning-ledger corrections.

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

This ledger is created before any product source changes per Plan 065 Phase 0.

## Phase 0.2 — Baseline

- starting HEAD: `805b305419e63970f3ee06b3b436e316cbf93513`
- working tree status: clean
- workspace members: `.`, `stegoeggo-stego`, `stegoeggo-cli`, `fuzz`
- root version (`Cargo.toml`): `0.3.2`
- carrier version (`stegoeggo-stego/Cargo.toml`): `0.3.2`
- CLI version (`stegoeggo-cli/Cargo.toml`): `0.3.2`
- root -> carrier dependency declaration (root `Cargo.toml`): `stegoeggo-stego = { path = "stegoeggo-stego" }` — note: does NOT specify an exact version `=0.3.2`. The release-check expects `={carrier_version}`.

## Phase 0.3 — Planning disposition

- `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md` is marked `PARTIAL — reopened by Plan 065` while this ledger is open.
- `plans/064-status.md` receives a correction note at top stating its COMPLETE disposition was superseded by the post-closure Plan 065 audit. Historical evidence below the note remains intact.
- Existing Plans 058-063 status files exist on disk under `plans/` but only `plans/064-status.md` is currently git-tracked (because `plans/` is `.gitignore`d). Phase 9 of this plan force-adds the retrospective ledgers.

## Status Rows

Each row starts `OPEN`. A row is closed only when the focused tests and source change described in its body have landed together with that row's acceptance criteria.

| ID | Row | Status | Evidence |
|----|-----|--------|----------|
| R01 | legacy Light default policy is `Unspecified` | CLOSED | `ProtectionLevel::default_policy()` and legacy/request tests |
| R02 | legacy Light maps to seed-only marker plan path | CLOSED BY 066 | `request_from_legacy()` maps Light to `HiddenMarkerMode::SeedOnly`; tile-size regression covered |
| R03 | legacy explicit DMI override preserved | CLOSED | legacy compatibility matrix |
| R04 | legacy metadata-injection three-state preserved | CLOSED | legacy compatibility matrix |
| R05 | legacy legal-claims `None`/`Some(false)`/`Some(true)` semantics | CLOSED BY 066 | legal-claim toggle tests and corrected docs |
| R06 | legacy explicit stego redundancy preserved into plan-driven embedding | CLOSED | legacy compatibility matrix |
| R07 | legacy content hash preserved into plan-driven v3 payload generation | CLOSED | legacy compatibility matrix |
| R08 | legacy timestamp override preserved | CLOSED | legacy compatibility matrix |
| R09 | legacy output/JPEG options + tile size + max dimension + metadata update policy + MAC key + resource limits + legal metadata preserved | CLOSED | legacy compatibility matrix |
| R10 | `ProtectionPipeline::process()` and `process_bytes()` delegate to canonical request/plan path | CLOSED BY 066 | stateless pipeline implementation and root suite |
| R11 | LSB pixel mutation never wraps `0 <-> 255` | CLOSED | carrier tests and full root suite |
| R12 | Default `stegoeggo-stego` API hides `jpeg_transcoder`, `JpegHeader`, `Coefficients`, `DctStegoF5`, etc. | CLOSED BY 066 | private modules, explicit root allowlist, compile-fail boundary doctests |
| R13 | Carrier `stegoeggo-stego` LSB helpers (permutations, bit helpers, embed_bit_in_pixel, seed-fallback) are not default public API | CLOSED BY 066 | private internals and operation-level support layer |
| R14 | Root `stegoeggo::stego` re-exports remain usable; root fuzzing still compiles under `fuzz` feature | CLOSED BY 066 | public API tests and bounded fuzz inspection path |
| R15 | LSB pixel-domain public API result boundary corrected (or recorded as deferred) | CLOSED | `RgbaImage` output boundary and examples/docs updated |
| R16 | Root `stegoeggo` declares carrier with path + exact `=0.3.2` version | CLOSED | root manifest exact path/version dependency |
| R17 | `stegoeggo-stego` package README/license/include declarations truthful | CLOSED | carrier README and package metadata audit |
| R18 | `cargo package -p stegoeggo-stego --allow-dirty` succeeds | CLOSED | staged release check |
| R19 | release-check version lockstep check agrees with manifests | CLOSED BY 066 | staged carrier → root → CLI release-check flow |
| R20 | `plans/058-status.md`, `059-status.md`, `060-status.md`, `061-status.md`, `063-status.md` exist, are git-tracked, and are labeled retrospective | CLOSED BY 066 | retrospective banners and forced staging |
| R21 | `plans/064-status.md` contains a correction/supersession note rather than an unqualified current COMPLETE claim | CLOSED BY 066 | final Plan 066 note |
| R22 | Roadmap 057 disposition corrected to `PARTIAL — reopened by Plan 065` while this plan is open | CLOSED BY 066 | final disposition is now `COMPLETE — final residuals closed by Plan 066` |
| R23 | Final `./scripts/check.sh` passes after all source corrections land | CLOSED | final local CI-equivalent check |
| R24 | Phase 10 focused test matrix passes (legacy compat, pipeline convergence, carrier correctness/API, packaging) | CLOSED | focused tests plus full workspace test |
| R25 | Documentation reconciliation: README, AGENTS.md, architecture/{overview,pipeline,protected-steganography,types}.md | CLOSED BY 066 | docs and boundary audit |

## Final closure reconciliation

The Phase 1–10 prose below is retained as the Plan 065 pre-change design and
audit record. Where it says `OPEN` or `BROKEN`, the final row table above and
Plan 066 are authoritative. Plan 066 corrected the remaining semantic issue
(Light plus tile size), removed the deleted public hook's production dependency,
contained the carrier boundary, made the pipeline stateless, and completed the
release/documentation ledger work.

## Phase 1 — Legacy compatibility matrix

Planned matrix (recorded before source changes):

| Legacy input | Required canonical meaning | Current state |
|--------------|----------------------------|---------------|
| `Disabled` | byte-for-byte passthrough | OK: `process_image_bytes` short-circuits when `Disabled` |
| `Light` default policy | `ProtectionLevel::default_policy()` -> `Unspecified` | BROKEN: `request_from_legacy` hard-codes `Light -> ProhibitedAllDataMining` instead of using `default_policy()` (see `src/lib.rs:828`) |
| `Standard` default policy | `ProhibitedAiMlTraining` | OK via `default_policy()` |
| explicit `DmiValue` | overrides level default | OK: `request_from_legacy` reads `ctx.dmi_value()` |
| `inject_metadata` | three-state preserved | OK: `effective_metadata_injection()` already handles `Some(false)` |
| `inject_legal_claims` | `None` defaults to auto-enable when `LegalMetadata` present; `Some(false)` disables; `Some(true)` enables | BROKEN: `request_from_legacy` uses `unwrap_or(true)` for the legal-claims decision. The three states are not distinguished |
| `seed` | exact propagation | OK |
| `intensity` | exact propagation | OK |
| explicit `stego_redundancy` | exact propagation | BROKEN: not represented in `ProtectionRequest` / `ResolvedProtectionPlan`. The `effective_redundancy_for_plan()` helper always derives from `plan.intensity()`. Explicit `ctx.stego_redundancy()` is lost |
| `content_hash` | exact propagation into v3 payload | BROKEN: `generate_payload_for_plan` hardcodes `let content_hash_8 = [0u8; 8];` whereas the legacy `generate_payload` reads `ctx.content_hash()` |
| output format | exact propagation | OK |
| JPEG quality | exact propagation | OK |
| progressive JPEG | exact propagation | OK |
| tile size | exact propagation | OK (mapped to `HiddenMarkerMode::Tiled`) |
| max dimension | exact propagation | OK |
| metadata update policy | exact propagation | OK |
| timestamp override | exact propagation | BROKEN: `request_from_legacy` builds the notice DMI/seed but ignores `ctx.timestamp_override()` for non-`LegalMetadata` paths; the canonical resolver does not consume `timestamp_override` either |
| MAC key | exact propagation + warning semantics | OK |
| legal metadata | injected only under same legacy conditions as before Plan 061 | BROKEN (same as inject_legal_claims) |
| resource limits | exact propagation | OK |

## Phase 2 — `HiddenMarkerMode::SeedOnly`

Current state: `HiddenMarkerMode` is `#[non_exhaustive]` with `Disabled | BestEffort | Tiled { tile_size }`. We will add `SeedOnly` so legacy Light can map to a canonical seed-only plan rather than relying on `Light -> BestEffort` plus `effective_redundancy_for_plan == 1`.

This is additive (`#[non_exhaustive]`) so it remains compatible for downstream exhaustive matches (modulo semver-introducing-a-variant which is documented as minor-bump-permitted).

## Phase 3 — Legacy field preservation

`ProcessingOptions` will gain:

```text
stego_redundancy: Option<usize>
content_hash: Option<[u8; 4]>
```

`ResolvedProtectionPlan` will expose these. `request_from_legacy` will copy from `ProtectionContext`.

`generate_payload_for_plan` will read `plan.processing().content_hash` instead of zeroing.

A single helper will replace the two `unwrap_or(true)` paths in `request_from_legacy` for legal-claim injection.

Timestamp override will be carried through `ProcessingOptions.timestamp_override: Option<String>` and consumed in the resolver.

## Phase 4 — `ProtectionPipeline` delegation

Public methods will delegate to the top-level `process_image` / `process_image_bytes` / `process_request_bytes` instead of running their own level-based pipeline. `Arc<...Protector>` fields may be removed if no callers need them.

## Phase 5 — Bounded LSB mutation

`embed_bit_in_pixel` will be rewritten to bound ±1:

```text
old == bit      -> new == old (no change)
old == 0        -> new == 1
old == 255      -> new == 254
1..=254         -> new == old - 1 or old + 1 by deterministic direction
```

The same rule will be applied to the seed-fallback path that currently re-implements the same mutation.

## Phase 6 — Carrier internal visibility containment

`jpeg_transcoder` will become `mod jpeg_transcoder;` (private). Public `jpeg` facade functions will use the type internally and return small public results.

Low-level LSB helpers will be demoted to `pub(crate)` unless explicitly justified. The `fuzz` feature path will be tightened so the fuzz target does not require exposing JPEG internals publicly under default features.

## Phase 7 — LSB result boundary

Phase 7.1 — current generic API has NOT been released on crates.io (verified by Phase 0.2 baseline; version is `0.3.2` and the prior plan explicitly records that the new API is not yet released). Therefore Phase 7.2 applies: we will correct the result boundary now. `lsb::embed` will return a typed result carrying an `RgbaImage` (or a typed wrapper); it will not internally PNG-encode.

## Phase 8 — Carrier package wiring

- Root `Cargo.toml`: change `stegoeggo-stego = { path = "stegoeggo-stego" }` to `stegoeggo-stego = { path = "stegoeggo-stego", version = "=0.3.2" }`.
- Carrier `Cargo.toml`: confirm `include` paths point only to files that exist.
- Add `stegoeggo-stego/README.md` (carrier-specific).
- Add `stegoeggo-stego/LICENSE` (MIT, same text as root).
- Verify `cargo package -p stegoeggo-stego --allow-dirty` and `--list` succeed.

## Phase 9 — Backfill ledgers

Force-add `plans/058-status.md`, `059-status.md`, `060-status.md`, `061-status.md`, `063-status.md`, `065-status.md`. Each will be clearly labeled retrospective.

## Phase 10 — Final closure

Run `./scripts/check.sh`. Update architecture docs and `AGENTS.md`. Only then mark Roadmap 057 COMPLETE.

## Acceptance Criteria

Plan 065 is complete when every blocking item in the plan §11 is true and all rows above are `CLOSED` with focused-test or source-audit evidence recorded.

## Implementation commits

To be filled in at closure.
