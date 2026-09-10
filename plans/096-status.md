# Plan 096 Status

Status: COMPLETE

Baseline: `27bdd3d429d663948021de43d3e6f818fa613319`
Depends on: Plans 091-095 as applicable.

## Required evidence

- [x] parent output/warning/verification baseline matrix recorded
- [x] public/hidden parent API v1 disposition table completed
- [x] parent-local application outcome vocabulary selected without unnecessary new public hierarchy
- [x] progressive/seed-only/best-effort fallback explicitly owned by parent
- [x] parent dogfoods strict/explicit generic carrier operations
- [x] `application-support` exports audited and reduced to evidenced legacy/application-specific needs
- [x] prepared JPEG replacement for hidden search context evaluated
- [x] `ProtectionPipeline` v1 keep/remove/redesign disposition implemented/documented at proper breaking boundary
- [x] `Protector` legacy-context disposition implemented/documented at proper breaking boundary
- [x] first-party current code no longer depends on deprecated config paths outside compatibility modules/tests
- [x] 0.x compatibility promises remain intact until explicit breaking version
- [x] root `stego` facade/direct-crate disposition documented
- [x] `./scripts/check.sh` passes (local full gate green, 2026-09-10)

## Implementation notes

- Baseline matrix: PNG/WebP standard LSB, JPEG baseline DCT success,
  lower-redundancy success, insufficient-capacity warnings, progressive
  seed-only fallback, tiled LSB/JPEG, metadata-only/seed-only, and legacy
  level/context equivalence are covered by `tests/request_api.rs`,
  `tests/cross_format_*.rs`, `tests/output_domain_routing.rs`,
  `tests/public_stego_api.rs`, and `tests/integration.rs`. All pass with
  the carrier changes; progressive suites re-verified explicitly because
  the seed hint is now transactional (standard quality-90 tables hold the
  full 96-bit hint, so fallback bytes are unchanged).
- Outcome vocabulary: no new public hierarchy. The parent keeps
  `EmbedOutcome`/`EmbedStatus`/`EmbedPath` as its application vocabulary
  (warnings, `ExecutionReport`, strict CLI behavior) while the generic
  carrier recommends `EmbedReport` + `StegoError` for new code. Converting
  `ExecutionReport` producers to a second new result type would add API
  without need; the mapping is documented at the carrier definitions.
- Fallback ownership: `embed_dct_payload`/`embed_dct_tiled_payload` use
  best-effort `carrier_jpeg::embed` as an explicit, documented application
  policy choice, with `progressive_fallback` calling `embed_seed_hint`
  directly. Short-table preflight failures now propagate as errors via `?`
  (consistent with malformed-input handling) instead of unrecoverable
  partial hints. Generic `embed_strict` is available but deliberately not
  used by the parent, whose best-effort contract is established behavior.
- Dogfooding: workspace grep confirms no production path passes runtime
  configuration through infallible `with_redundancy` (parent uses
  `try_new`; remaining `with_redundancy` uses are validated constants in
  tests/benches/examples/docs). `ProtectionContext` constructions in
  production are the legacy adapters themselves (`types/compat.rs`,
  `request_from_legacy`, `Protector` impls) plus one plan-side translation
  adapter (`metadata_trap.rs` `limits_ctx`, now documented as such).
  Threading scalar level/seed/intensity/limits through the shared JPEG
  injection internals was evaluated and rejected for this plan: the
  structured-COM renderer reads all three fields and any signature change
  risks canonical metadata bytes. Full removal of the legacy context from
  those internals belongs to the explicit v1 boundary (see table).
- `application-support`: 18 public symbol lines before and after (6
  compat functions, `tile_seed`, 3 search/candidate types, 8 methods).
  Reduction evaluated per symbol: legacy V1 extractors, LSB seed
  fallback, tile-seed derivation, `TiledJpegSearch`/
  `TiledJpegCandidateKey`, and `JpegSearchContext` each serve retained
  verification compatibility or application candidate classification that
  the generic API deliberately excludes (Plans 094/095 deltas: stale
  `PRIVATE-REUSE-SUFFICIENT` wording updated; no exact duplicate of
  `PreparedJpeg`/pixel views exists in hidden support). Nothing removed
  because nothing was redundant; the audit is the evidence.
- Root facade: `stego::{pixels, prepared}` modules and
  `Redundancy`/`PreparedJpeg`/`PixelLayout`/`PixelView`/`PixelViewMut`
  re-exports added. `stegoeggo-stego` documented as canonical for
  generic-only use; the facade is a convenience subset with no exclusive
  carrier API.

## v1 disposition table

| Item | Current status | Canonical replacement | 0.x | v1 disposition |
|---|---|---|---|---|
| `ProtectionPipeline` | Does not exist as a type; canonical execution is free functions in `pipeline.rs` (`execute_*`) | n/a (already function-based) | n/a | Do not introduce an empty wrapper; keep functions |
| `Protector` trait (`traits.rs`, context/level-based) | Public, implemented by all protectors | `ProtectionRequest` + top-level `process_request_bytes*` | Keep | Remove at breaking boundary; do not rename — design a request-oriented extension contract only if a third-party use case is demonstrated |
| `SteganographyProtector` | Public, `pub(crate)` behavior split across 5 modules | Canonical request functions for ordinary use | Keep | Keep only if it exposes coherent standalone ops; else internalize at boundary |
| `RightsMetadataProtector` | Public metadata injector | Same (it *is* the canonical metadata path) | Keep | Keep |
| `MetadataTrapProtector` | Deprecated alias | `RightsMetadataProtector` | Keep (deprecated) | Remove at boundary |
| `PassthroughProtector` | Public no-op strategy | Same | Keep | Keep |
| `ProtectionContext` | Public legacy config (not attribute-deprecated) | `ProtectionRequest`/`ProcessingOptions` | Keep | Remove or internalize at boundary; internal JPEG injection signatures must take explicit limits/identity params first |
| `ProtectionLevel` | Deprecated compat adapter | `RightsPolicy` + presets | Keep (deprecated) | Remove at boundary |
| `EvidenceProfile` | Deprecated compat adapter | `ProtectionPreset` | Keep (deprecated) | Remove at boundary |
| Root `stego` facade | Public convenience re-export | Direct `stegoeggo-stego` dependency | Keep | Keep facade, keep preferring direct crate in docs |
| Carrier `EmbedOutcome` family | Public, parent-owned semantics | `EmbedReport` + `StegoError` for generic code | Keep | Remove from recommended generic surface at boundary; parent keeps its own vocabulary |
| `application-support` | Hidden, 18 symbols | Public `prepared`/`pixels`/strict APIs where generic | Keep hidden | Keep hidden; promote nothing without a generic use case |

Record final `./scripts/check.sh` result and implementation commit SHA here
during closure.

Implementation commit SHA: `d42f8ba` (roadmap 090 implementation on `main`; this ledger closure is the follow-up commit).
