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

## Re-verification (2026-09-10)

- Source re-audited against all acceptance criteria: validated `Redundancy`
  primitive (`MIN`/`MAX`, `new`/`from_usize`, `Copy`, no `Deref`) as the
  recommended config primitive; `from_redundancy`/`with_redundancy_value`/
  `redundancy_value` on both configs; infallible `with_redundancy` retains
  exact legacy debug-assert/release-clamp behavior as a documented
  constant-only compatibility builder (no `#[deprecated]` attribute: first-
  party tests/benches/examples/docs use it with validated constants only,
  and attribute deprecation would fail `-D warnings`); parent production
  paths use `try_new` (verified by workspace grep); `TileConfig` keeps its
  fallible private-field shape with JPEG multiple-of-8/minimum-8 validation
  at the operation boundary; distinct-method strict/best-effort split from
  Plan 091 is the explicit JPEG capacity policy (no policy enum or boolean
  needed); `EmbedReport` + `StegoError` is the recommended 0.x generic
  result (public fields frozen through 0.x, private-fields-plus-getters at
  the v1 boundary) with `into_output`/`into_parts`/`capacity` helpers and
  `InPlaceEmbedReport` vocabulary parity; `EmbedOutcome` family retained as
  documented parent-owned application vocabulary; capacity units exact
  (`|coef| >= 2`, RGB slots, 96 hint-bit positions); zero seeds valid with
  round-trip tests on both carriers; no generic `Carrier` trait added.
- Doc corrections in this pass: carrier README double-`?` typo in the
  `from_redundancy` example; `EmbedReport` rustdoc (frozen-public-fields
  through 0.x, v1 getter direction); `architecture/protected-steganography.md`
  (`with_redundancy` compatibility-builder wording, `EmbedReport` freeze
  note); `docs/carrier-crate.md` (debug-assert/release-clamp adapter
  wording); `STABILITY.md` (`EmbedReport`/`InPlaceEmbedReport` field-freeze
  plus v1 getter direction). README, AGENTS.md, skills, and
  `DEPRECATIONS.md` already stated the 092 contract; no changes needed there.
- `./scripts/check.sh` re-run during this pass (exit 0).
