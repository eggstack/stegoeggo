# Plan 092 Status

Status: COMPLETE

Baseline: `27bdd3d429d663948021de43d3e6f818fa613319`
Depends on: Plan 091 semantic disposition.

## Required evidence

- [x] complete public config/result/root-reexport semver inventory recorded
- [x] validated shared redundancy representation implemented
- [x] recommended runtime configuration has identical debug/release semantics
- [x] valid legacy/new configs produce identical carrier bytes
- [x] zero-seed behavior/documentation reconciled
- [x] JPEG capacity policy represented explicitly
- [x] future-proof carrier-neutral recommended result model implemented
- [x] application-only progressive/Q-table fallback states absent from recommended generic result
- [x] 0.x compatibility/deprecation mapping documented
- [x] public traits/docs/doctests audited
- [x] `./scripts/check.sh` passes (local full gate green, 2026-09-10)

## Implementation notes

- Semver inventory: public config/result surface is `LsbConfig`,
  `JpegConfig`, `TileConfig`, `MAX_TILED_ORIGINS`, `EmbedReport`,
  `InPlaceEmbedReport`, `EmbedOutcome`, `EmbedOutcomeSummary`,
  `EmbedPath`, `EmbedStatus`, `CapacityReport`, all re-exported at the
  carrier root and via `stegoeggo::stego`. All changes here are additive:
  no signature removed, no public field changed, no default altered.
- `Redundancy` (`types.rs`, re-exported at carrier root): private `u8`
  representation, `Copy`, `MIN`/`MAX` consts, `new(u8)` /
  `from_usize(usize)`, `get()` / `get_usize()`, `TryFrom<u8>` /
  `TryFrom<usize>`, `From<Redundancy>` for `u8`/`usize`, `Display`, full
  `Clone/Copy/Debug/Eq/PartialEq/Ord/PartialOrd/Hash`. `Deref` deliberately
  not implemented.
- `LsbConfig`/`JpegConfig` gained `from_redundancy(seed, Redundancy)`,
  `with_redundancy_value(Redundancy)`, and `redundancy_value()`.
  Compatibility decision: infallible `with_redundancy(usize)` keeps its
  exact legacy behavior (debug assert + release clamp) through 0.x and is
  documented as a compile-time-constant compatibility builder; it is not
  given a `#[deprecated]` attribute because first-party tests, benches,
  examples, and docs use it exclusively with validated constants and
  attribute deprecation would fail `-D warnings`. Runtime values must use
  `try_*` or the `Redundancy` setters. No first-party production path
  passes runtime configuration through `with_redundancy` (parent uses
  `try_new`; verified by workspace grep).
- `TileConfig` keeps its private-field fallible shape; JPEG
  multiple-of-8/minimum-8 validation stays at the JPEG operation boundary
  (`validate_jpeg_tile_size`), not in generic `TileConfig`.
- JPEG capacity policy is represented by the Plan 091 distinct-method
  split (`embed`/`embed_framed` best-effort compatibility vs
  `embed_strict`/`embed_framed_strict` exact); no policy enum or boolean
  parameter was needed, so none was added.
- Result model: `EmbedReport` is the recommended carrier-neutral result
  with new decomposition helpers (`into_output`, `into_parts`,
  `capacity()`); failures use `StegoError` rather than sentinel reports.
  `EmbedOutcome`/`EmbedStatus`/`EmbedPath`/`EmbedOutcomeSummary` are
  retained unchanged for 0.x compatibility and documented as
  parent-owned application surfaces. No generic `Carrier` trait added.
- Capacity units made exact everywhere in the carrier crate
  (`|coef| >= 2` for DCT; RGB slots for LSB; 96 hint-bit positions for
  seed hints). `StegoError::InvalidConfig` no longer cites zero seed as an
  example; zero seeds are explicitly valid with round-trip tests on both
  carriers.
- Tests: full `1..=10` construct/convert round-trip, deterministic
  rejection of `0/11/overflow` in both widths, legacy/new byte-equivalence
  across all redundancies (LSB and JPEG incl. strict), zero-seed
  round-trips, report-helper preservation, doctests for every new public
  constructor/setter against the direct crate path.
- No algorithm byte change, no frame/payload version change, no new
  public trait, no error-framework rewrite.

Record final `./scripts/check.sh` result and implementation commit SHA here
during closure.

Implementation commit SHA: `d42f8ba` (roadmap 090 implementation on `main`; this ledger closure is the follow-up commit).
