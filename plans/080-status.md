# Plan 080 Status Ledger: Stego Pipeline, Library Evidence, and Documentation Closure

Created before any corrective product edit, per Plan 080 Phase 0.1.

## Baseline

- starting HEAD: `def20777f76875470f004fa37db4b8ff75f072a7`
  (`plan 079: public tiled carrier API and parent dogfooding`)
- audited planning baseline from plan text: `main` at
  `6feb52a90d9afdc0c922cdb219529524ad94c168`
- working tree status at ledger creation: clean
  (`git status --porcelain` empty, branch `main`)
- workspace versions: `stegoeggo 0.4.0`, `stegoeggo-stego 0.4.0`,
  `stegoeggo-cli 0.4.0`
- Plan 077 implementation/status commit: `16858ce`
  (`plan 077: output-domain carrier routing correctness`)
- Plan 078 implementation/status commit: `8b1c47b`
  (`plan 078: single-decode and tiled allocation optimization`)
- Plan 079 implementation/status commit: `def2077`
  (`plan 079: public tiled carrier API and parent dogfooding`)
- root -> carrier dependency declaration: `stegoeggo-stego = { path =
  "stegoeggo-stego", version = "=0.4.0", features = ["application-support"] }`
  (root `Cargo.toml:48`); CLI depends on root via `stegoeggo = { path = "..",
  version = "=0.4.0", features = ["iscc", "conformance", "parallel"] }`
- current Roadmap 076 status: `Ready for implementation`, no implementation
  plan marked complete (077/078/079 all still `Ready` at roadmap header level)

## Requirement rows

| ID | Requirement | Status |
|----|-------------|--------|
| R01 | output-domain carrier invariant verified from source and tests | OPEN |
| R02 | full input/output format matrix verified | OPEN |
| R03 | same-format JPEG encoded-byte fast path verified | OPEN |
| R04 | non-JPEG raster processing uses one full pixel decode | OPEN |
| R05 | same-format metadata-only container path avoids pixel decode where supported | OPEN |
| R06 | tiled LSB parent path avoids second full-image clone | OPEN |
| R07 | tiled JPEG embed performs one coefficient decode | OPEN |
| R08 | standard JPEG application verification decode count meets Plan-078 disposition | OPEN |
| R09 | standard+tiled JPEG verification reuse meets recorded disposition | OPEN |
| R10 | verification search coverage/resource limits preserved | OPEN |
| R11 | public raw carrier APIs verified | OPEN |
| R12 | public framed carrier APIs verified | OPEN |
| R13 | public in-place carrier APIs verified | OPEN |
| R14 | public tiled carrier APIs verified | OPEN |
| R15 | prepared JPEG API disposition is evidence-consistent | OPEN |
| R16 | parent current operations dogfood stable public carrier API where intended | OPEN |
| R17 | application_support residual surface is justified and narrow | OPEN |
| R18 | JPEG implementation structs remain private | OPEN |
| R19 | LSB implementation helpers remain private | OPEN |
| R20 | standalone direct carrier consumer compiles | OPEN |
| R21 | root stego re-export consumer compiles | OPEN |
| R22 | docs/examples match source signatures and behavior | OPEN |
| R23 | architecture docs match actual routing/decode ownership | OPEN |
| R24 | benchmark/decode/allocation evidence is reproducible | OPEN |
| R25 | cargo package/release structural checks remain healthy | OPEN |
| R26 | full workspace tests pass | OPEN |
| R27 | ./scripts/check.sh passes | OPEN |
| R28 | no unapproved dependency/version/release/CI change | OPEN |
| R29 | Roadmap 076 final disposition is truthful | OPEN |

## Audit notes

(Updated as evidence lands; every closure cites files/lines and commands.)

## Phase 1 — independent source architecture audit

### 1.1 Output-domain carrier invariant (R01)

Canonical selection in `src/lib.rs`:

- `execute_full_marker_and_metadata` (`src/lib.rs:1238`): `input == Jpeg &&
  output == Jpeg` takes the encoded-byte DCT fast path
  (`apply_dct_stego_bytes_from_plan`); `output == Jpeg` after a pixel decode
  re-encodes then DCT-embeds (`src/lib.rs:1254-1269`); otherwise raster LSB
  via `apply_lsb_to_image_with_summary_from_plan` (`src/lib.rs:1272-1273`).
- `execute_seed_only_and_metadata` (`src/lib.rs:1298`, `1308`): JPEG/JPEG
  Q-table seed, JPEG-output Q-table seed after encode, else LSB seed fallback.
- `apply_lsb_to_image_with_summary_from_plan`
  (`src/protected/steganography/embed.rs`): raster-only; `EmbedPath` from
  `tile_size` only. `apply_dct_stego_bytes_from_plan`: JPEG-only; `EmbedPath`
  from `tile_size` only.

`input_format` remaining uses (all classified, none is current-path
carrier-family selection):

- encoded-byte reuse: `src/lib.rs:1024` (JPEG header inspect gate),
  `src/lib.rs:1064/1074/1236/1238/1292/1298` (JPEG/JPEG fast-path skip),
  `src/protected/steganography/mod.rs:238` (trait-default re-encode hint)
- format conversion: `src/lib.rs:430/452` (legacy pixel path format
  propagation), `src/lib.rs:963` (`format_transcoded` flag),
  `src/lib.rs:1104/1108` (metadata-only transcode decision),
  `src/traits.rs:42` (trait default re-encode)
- validation: `src/lib.rs:536/760/862/911/952` (plan resolution, magic-byte
  detection), `resolve_request` test fixtures
- benign legacy/test-only: `embed.rs:418-420`
  (`apply_to_image_with_summary`, legacy pixel path where the caller sets
  `input_format` to the output format) and `marker.rs:153/158`
  (`generate_payload_for_context`, test-only; canonical
  `generate_payload_for_plan` receives `EmbedPath` from the caller)

Verdict: no current-path carrier decision uses `input_format`. Carrier family
reduces to `output JPEG -> DCT/F5`, `output PNG/WebP -> raster LSB`. R01
evidence-backed.

### 1.2 Decode ownership table (R04-R09)

Source-verified against `def2077` (`src/lib.rs:1016-1096`,
`1228-1345`; `embed.rs`; `extract.rs`; `stegoeggo-stego/src/jpeg.rs`,
`application_support.rs`, `lsb_internal.rs`):

| Path | Pixel decodes | Coefficient decodes | Full-image clones | Notes |
|---|---:|---:|---:|---|
| JPEG->JPEG BestEffort | 0 | 1 | 1 coeff clone | fast path `src/lib.rs:1238-1248`; `jpeg::embed` decodes once, clones coefficients for mutation |
| JPEG->JPEG Tiled | 0 | 1 | 0 | `jpeg::embed_tiled` decodes once, mutates in place, self-checks mutated memory |
| JPEG verify standard | 0 | 1 per verification op | 0 | one `JpegSearchContext` per op (`extract.rs:595-601`); up to 2 ops per verify (Q-table seed, then metadata seed) |
| JPEG verify standard+tiled fallback | 0 | 1 per verification op (shared) | 0 | `verify_extract_dct_with_seed` reuses the context for tiled fallback |
| JPEG->PNG/WebP | 1 | 0 | 0 | one `load_image_from_bytes` (`src/lib.rs:1250`) + in-place LSB (`src/lib.rs:1272`); never a transient DCT step |
| PNG/WebP->same raster | 1 | 0 | 0 | one decode; `to_rgba8` buffer mutated in place |
| PNG/WebP->JPEG | 1 | 1 | 1 coeff clone | pixel decode, encode to JPEG (`src/lib.rs:1255-1260`), DCT embed decodes+clones |
| tiled raster parent | 1 | 0 | 0 | owned RGBA mutated via `embed_lsb_tiled_in_place`; cloning `embed_lsb_tiled` delegates to the core |
| metadata-only same-format | 0 | 0 | 0 | `inject_bytes_from_plan` on raw bytes (`src/lib.rs:1121`) |
| metadata-only cross-format | 1 | 0 | 0 | decode + re-encode (`src/lib.rs:1109-1118`) |

Preflight is header-only on all paths: JPEG via `stego::jpeg::inspect`
(`src/lib.rs:1024-1038`); non-JPEG via `ImageReader::into_dimensions`
(`src/lib.rs:1039-1052`). The Plan-078 discarded full decode is gone.

Correction recorded: an early draft of this table claimed JPEG->PNG/WebP
performs a coefficient decode via transient-JPEG DCT. That is false under the
Plan-077 routing — those paths take raster LSB with zero coefficient
decodes (proven by `tests/output_domain_routing.rs` `EmbedPath::Lsb`
assertions).

### 1.3 Parent/carrier boundary (R16, R17)

`stegoeggo-stego/src/application_support.rs` (feature-gated hidden) exports 9
symbols, each with module-doc rationale:

- LEGACY-COMPATIBILITY: `legacy_lsb_required_slots`, `legacy_lsb_extract`,
  `legacy_lsb_extract_range` (V1 only; promoting would misrepresent a
  superseded format)
- APPLICATION-SPECIFIC-SEARCH: `seed_fallback_embed`,
  `seed_fallback_extract` (historical seed compat), `tile_seed` (application
  candidate seed derivation), `TiledJpegSearch`/`TiledJpegCandidateKey`
  (application bounded candidate classification), `JpegSearchContext`
  (single-decode verification reuse, Plan-078 `PRIVATE-REUSE-SUFFICIENT`)

Root `carrier_support::` call sites (`embed.rs:64/153/356/360`,
`extract.rs:27/55/60/597/600/605/618/673/692/839/857/881/1335-via-local`,
`mod.rs:178`) classify identically. Zero TEMPORARY-RESIDUAL: ordinary
current LSB/JPEG/tiled embed/extract route through stable `carrier_lsb::`
(`embed.rs`) / `carrier_jpeg::` and `lsb::extract`, with `crop_rgba` now
root-local. R16/R17 evidence-backed.

## Phase 2 — behavior matrix and regression verification (R02, R03, R10, R14)

Measured on `def2077` (`cargo test -p <pkg> --all-features`):

- `output_domain_routing` 17/17 pass: BestEffort 3x3 matrix, Tiled 9-case
  matrix, 4 SeedOnly direction tests. Every case asserts requested output
  magic, `format_transcoded`, `EmbedPath` matching final format, final-byte
  verification, and rights metadata presence.
- `request_api` 80/80, `cross_format_semantics` 34/34,
  `cross_format_closure` 25/25 pass.
- `public_stego_api` 52/52 pass (incl. 7 Plan-079 tiled-via-`stegoeggo::stego`
  tests; R14 covered from the consumer side).
- `plan065_legacy_compat` 10/10, `plan026_gate1_2_3_tests` 57/57 pass —
  V1/V2 and deprecated-compat images still verify (R10-compatible legacy
  coverage).
- `verification_report_tests` 19/19 pass.
- Name-filter suites: `tiled` (2+7+1 across lib/integration/doc targets),
  `jpeg` (7+25+1) all pass, zero failures.
- `stegoeggo-stego --all-features`: lib + 25 doctests pass (incl. new
  `TileConfig`/`embed_tiled` doctests and retained `jpeg_transcoder` /
  `lsb_internal` compile-fail boundary doctests).

SeedOnly output-domain behavior is covered independently by the 4 SeedOnly
direction tests in `output_domain_routing` (JPEG-hint vs raster-fallback
matrix). R02/R03 verified; R10/R14 green so far (full workspace run in
Phase 7).

## Phase 3 — standalone generic carrier consumer proof (R11-R14, R20, R21)

- Direct consumer `/tmp/stego080-consumer` (path dep on `stegoeggo-stego`,
  default features, `application-support` NOT enabled; not committed):
  `cargo run --release` prints `standalone carrier consumer: all operation
  styles ok`. Covered: LSB raw, LSB in-place, LSB framed, LSB tiled
  raw/framed (`TileConfig::try_new(42, 64)`), JPEG `probe_support` +
  `inspect` + `is_progressive_jpeg` + `capacity`, JPEG raw (report
  `actual_redundancy` fed to `extract`), JPEG framed, JPEG tiled raw/framed.
- Root re-export consumer `/tmp/stego080-rootconsumer` (path dep on
  `stegoeggo`, uses only `stegoeggo::stego`; not committed): `cargo run
  --release` prints `root stego re-export consumer: all operation styles
  ok` (LSB raw/in-place/framed/tiled + JPEG raw/framed). The root facade
  exposes the full intended surface (module audit in Phase 1.3: all 5
  modules + 12 types/constants re-exported; `constants` correctly excluded).
- Dependency surface (`cargo tree -p stegoeggo-stego --edges normal`):
  direct deps are `crc32fast`, `image` (png+jpeg only), `jpeg-encoder`,
  `thiserror`. No rights/provenance/metadata/policy dependency leaks into
  the carrier package.
- Consumer note: an early consumer draft called
  `lsb::extract_tiled_framed` on raw-tiled output and got `FrameNotFound`
  (correct: raw output carries no frame), and JPEG-encoding an RGBA8 image
  via `image` fails with `Unsupported(Color(Rgba8))` (convert to RGB8
  first). Both are consumer-side expectations, not library defects.

## Phase 4 — private-boundary and semver audit (R15, R18, R19)

- `stegoeggo-stego/src/lib.rs:44/46`: `jpeg_transcoder` and `lsb_internal`
  are `pub(crate)`; `application_support` is `#[cfg(feature =
  "application-support")]` + `#[doc(hidden)]` (`:49-51`).
- Compiler-proven privacy: `compile_fail` doctests (`lib.rs:28-34`) for
  `jpeg_transcoder::JpegTranscoder` and `lsb_internal::stego_permutation_v2`
  pass under `cargo test -p stegoeggo-stego --doc` (25/25 green).
- `JpegSearchContext` holds one private `decoded: DecodedJpegCarrier`
  (`application_support.rs:115`, `DecodedJpegCarrier` is `pub(crate)`);
  public methods expose only `usize`/`u64`/`u32`/`Vec<u8>`/`StegoError`/
  `CapacityReport`/`TiledJpegCandidateKey` — no coefficient/header/F5 types,
  no debug/deref leaks.
- Public API review: `TileConfig` private fields + `try_new` rejection of
  `0` + `#[must_use]` getters + `MAX_TILED_ORIGINS = 4096` with
  `validate_max_origins` (`1..=4096`); `LsbConfig`/`JpegConfig` builders
  carry `#[must_use]`; fallible `try_new`/`try_with_redundancy` on both;
  JPEG tiled rejects `< 8` / non-multiple-of-8 tile sizes; consistent
  `StegoError` variants; bounded `max_origins` on all four tiled extracts;
  capacity reports in slot/byte units as before. No `#[non_exhaustive]`
  added, per crate convention.
- Root dogfooding (Phase 1.3 call-site audit): ordinary current operations
  route through stable `lsb::`/`jpeg::`; hidden support holds only
  LEGACY-COMPATIBILITY and APPLICATION-SPECIFIC-SEARCH.
- Prepared JPEG disposition: `NO-PROMOTION` (carried from Plan 078
  `PRIVATE-REUSE-SUFFICIENT`; re-confirmed — the only multi-operation reuse
  is application verification search via hidden `JpegSearchContext`; generic
  repeated ops are served by one-shot `extract_framed` with one retained
  decode per call). No prepared type exists, so no prepared benchmark is
  owed. R15 evidence-consistent.

## Phase 5 — performance evidence closure (R04-R09, R24)

Deterministic counts re-confirmed on `def2077` (no timing noise):

- JPEG standard verification == 1 coefficient decode per verification op:
  `jpeg_search_context_decodes_once_for_standard_and_tiled_probes` (20
  standard extracts + capacities + full tiled scan == 1 decode) plus root
  `plan078_standard_jpeg_verification_uses_single_search_context` (1
  context per verification) pass.
- Standard+tiled fallback == same single decode: root
  `plan078_tiled_fallback_jpeg_verification_uses_single_search_context`
  passes; stego `tiled_jpeg_search_decodes_once_for_multiple_candidate_extractions`
  and `tiled_jpeg_search_no_match_broad_scan_decodes_once` pass.
- JPEG tiled embed == 1 decode + 1 encode: stable-path
  `jpeg::tests::tiled_framed_decodes_once_per_operation` passes (Plan-078
  hidden-path counter migrated to the stable fixture in Plan 079; evidence
  retained in `078-status.md`).
- LSB tiled in-place == cloning behavior with no parent clone:
  `tiled_in_place_roundtrip_matches_cloning`,
  `tiled_in_place_matches_cloning_embed`,
  `tiled_in_place_report_matches_cloning_capacities`, both
  `..._leaves_carrier_unchanged_on_insufficient_capacity` pass.
- Raster bytes-in/bytes-out == 1 pixel decode; metadata-only same-format ==
  0: root `plan078_png_marker_performs_single_pixel_decode` and
  `plan078_metadata_only_png_performs_no_pixel_decode` pass.

Wall-clock (local Criterion `--quick`, textured fixtures, no CI gates —
structural claims rest on the counts above):

- `jpeg_standard_verify`: 256 ~5.9 ms, 1024 ~90.7 ms.
- `tiled_lsb_request/png_tiled/1024`: ~30.9 ms.
- `jpeg_fast_path_512x512/jpeg_in_out/standard`: ~1.09 ms.
- `process_bytes/png_512/standard`: ~4.75 ms.
- `lsb_clone_vs_in_place`: in-place 1024 ~65.5 us vs clone path; 4096
  in-place ~104 us vs clone ~1.16 ms.

Methodology uses equivalent pristine textured inputs per group; setups
assert `Verified` where the group measures a success path (per 078 notes).
No CI bench thresholds added. R24 reproducible (counts deterministic;
benches re-runnable via `cargo bench --bench bench -- --quick <group>`).

## Phase 6 — documentation and examples reconciliation (R22, R23)

Independent audit (grep for `input_format`, `input-domain`, `prepared` /
`PreparedJpeg`, `application_support`, per-redundancy `jpeg_extract`,
`re-decode`, `transient JPEG` across all listed docs plus stale
helper/executor names): no stale input-domain carrier description, no stale
executor/helper names, no hidden-support API presented as stable, security
wording intact (`carrier-crate.md:29`, `stegoeggo-stego/README.md:50-51`,
`:235-246` — CRC32 corruption-only, seed not secret, no
steganalysis-resistance, lossy/transcode fragility).

Corrective doc edits (this plan; product source untouched):

- `architecture/overview.md:361/363` — `lsb.rs`/`jpeg.rs` module-map rows
  now include tiled operations.
- `docs/rust-api.md:72` — `generic_stego.rs` example row now lists tiled.
- `README.md:34` — dependency snippet `stegoeggo = "0.3"` -> `"0.4"`.
- `STABILITY.md` — removed contradicting `VerificationStatus ->
  VerificationReport` deprecation row (contradicted `DEPRECATIONS.md:20`
  and `AGENTS.md:111`: `VerificationStatus` is NOT deprecated); added a
  `Generic Carrier API` stable tier (raw/in-place/framed/tiled via
  `stegoeggo::stego` / `stegoeggo-stego`, no public prepared type);
  clarified `jpeg_transcoder/` is private to `stegoeggo-stego`.
- `CHANGELOG.md` `[Unreleased]` — `Added`: public tiled carrier operations
  + standalone-consumer proof; `Changed`: output-domain routing correction
  + single-decode/single-clone optimizations (shipped in 077-079,
  previously unrecorded).

No change needed (verified current): `stegoeggo-stego/README.md` and
`docs/carrier-crate.md` (full tiled coverage + security wording),
`architecture/{pipeline,protected-steganography,jpeg-stego-f5,jpeg-transcoder}.md`
(output-domain invariant, single-decode flow, in-memory self-check,
NO-PROMOTION at `protected-steganography.md:343`), `AGENTS.md` (routing,
`JpegSearchContext`, `TileConfig`/`max_origins` gotchas current; plan
number `081+` still correct), `.skills/stegoeggo-conventions/SKILL.md`
(pitfalls 11/13/14/17/18/22 current, incl. tiled signatures at `:141-164`).

Examples: `cargo run --example generic_stego --all-features` succeeds and
demonstrates tiled LSB + tiled JPEG round-trips. R22/R23 closed.

## Phase 7 — package and repository gates (R25, R26, R27, R28)

Measured on final tree (docs edits only; no product-source change in this
plan):

- `cargo fmt --all -- --check`: pass (exit 0).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
  pass.
- `cargo check -p stegoeggo --no-default-features`: pass.
- `cargo check -p stegoeggo-stego` (default) and `--features
  application-support`: pass.
- `cargo test -p stegoeggo-stego --all-features` + `--doc` (25/25): pass.
- Focused: `output_domain_routing` 17/17, `request_api` 80/80,
  `cross_format_semantics` 34/34, `cross_format_closure` 25/25,
  `public_stego_api` 52/52, `plan065_legacy_compat` 10/10,
  `plan026_gate1_2_3_tests` 57/57, `verification_report_tests` 19/19,
  `tiled`/`jpeg` name-filter suites all pass.
- `cargo test --workspace --exclude stegoeggo-fuzz --all-features`: 1803
  passed, 0 failed across 40 targets.
- `./scripts/check.sh`: exit 0.
- `./scripts/release-check.sh --allow-dirty --skip-check --stage=pre`:
  `=== Release check passed ===` (no publication; structural/package
  checks healthy, version lockstep `0.4.0` intact).
- R28: diff touches only docs/plans (`README.md`, `STABILITY.md`,
  `CHANGELOG.md`, `architecture/overview.md`, `docs/rust-api.md`,
  `plans/080-status.md`, roadmap header); no `Cargo.toml`/`Cargo.lock`,
  version, dependency, or workflow change.

## Phase 8 — final roadmap disposition

All rows R01-R29 CLOSED (evidence cited per phase above). No
TEMPORARY-RESIDUAL, no corrective product patch required, no new numbered
corrective plan needed.

Roadmap 076: COMPLETE.

- Plan 077 routing correction disposition: complete (`16858ce`;
  output-domain carrier routing with 17-test regression matrix).
- Plan 078 measured optimization disposition: complete (`8b1c47b`;
  single-decode verification/embed, in-place tiled LSB, header-only
  preflight; prepared-API disposition `PRIVATE-REUSE-SUFFICIENT`).
- Plan 079 public API/prepared-object disposition: complete (`def2077`;
  public tiled carrier API + root dogfooding; prepared-API disposition
  `NO-PROMOTION`).
- Plan 080 final verification commit: this commit (evidence ledger +
  documentation closure; zero product-source edits).

## Requirement rows — final

| ID | Requirement | Status |
|----|-------------|--------|
| R01 | output-domain carrier invariant verified from source and tests | CLOSED (Phase 1.1; `output_domain_routing` 17/17) |
| R02 | full input/output format matrix verified | CLOSED (Phase 2; 3x3 BestEffort + 9-case Tiled + SeedOnly) |
| R03 | same-format JPEG encoded-byte fast path verified | CLOSED (Phase 1.2/2; 0 pixel + 1 coeff decode; `jpeg_fast_path` bench) |
| R04 | non-JPEG raster processing uses one full pixel decode | CLOSED (Phase 5; `plan078_png_marker_performs_single_pixel_decode`) |
| R05 | same-format metadata-only container path avoids pixel decode where supported | CLOSED (Phase 5; `plan078_metadata_only_png_performs_no_pixel_decode`) |
| R06 | tiled LSB parent path avoids second full-image clone | CLOSED (Phase 5; in-place core identity tests) |
| R07 | tiled JPEG embed performs one coefficient decode | CLOSED (Phase 5; `tiled_framed_decodes_once_per_operation`) |
| R08 | standard JPEG application verification decode count meets Plan-078 disposition | CLOSED (Phase 5; 1 decode/op, context-count tests) |
| R09 | standard+tiled JPEG verification reuse meets recorded disposition | CLOSED (Phase 5; shared-context tests) |
| R10 | verification search coverage/resource limits preserved | CLOSED (Phase 2; legacy/compat/verification suites green) |
| R11 | public raw carrier APIs verified | CLOSED (Phase 3; external consumer raw LSB+JPEG) |
| R12 | public framed carrier APIs verified | CLOSED (Phase 3; external consumer framed LSB+JPEG) |
| R13 | public in-place carrier APIs verified | CLOSED (Phase 3; external consumer in-place) |
| R14 | public tiled carrier APIs verified | CLOSED (Phase 2/3; `public_stego_api` 52/52 + external tiled) |
| R15 | prepared JPEG API disposition is evidence-consistent | CLOSED (Phase 4; `NO-PROMOTION`, no prepared type exists) |
| R16 | parent current operations dogfood stable public carrier API where intended | CLOSED (Phase 1.3; zero residual call sites) |
| R17 | application_support residual surface is justified and narrow | CLOSED (Phase 1.3; 9 exports, all classified) |
| R18 | JPEG implementation structs remain private | CLOSED (Phase 4; `pub(crate)` + compile-fail doctests) |
| R19 | LSB implementation helpers remain private | CLOSED (Phase 4; `pub(crate)` + compile-fail doctests) |
| R20 | standalone direct carrier consumer compiles | CLOSED (Phase 3; `/tmp/stego080-consumer` all-styles ok) |
| R21 | root stego re-export consumer compiles | CLOSED (Phase 3; `/tmp/stego080-rootconsumer` all-styles ok) |
| R22 | docs/examples match source signatures and behavior | CLOSED (Phase 6; 5 corrective edits + example run) |
| R23 | architecture docs match actual routing/decode ownership | CLOSED (Phase 6; audit clean, module-map rows fixed) |
| R24 | benchmark/decode/allocation evidence is reproducible | CLOSED (Phase 5; deterministic counts + re-runnable benches) |
| R25 | cargo package/release structural checks remain healthy | CLOSED (Phase 7; release-check pre passed) |
| R26 | full workspace tests pass | CLOSED (Phase 7; 1803 passed, 0 failed) |
| R27 | ./scripts/check.sh passes | CLOSED (Phase 7; exit 0) |
| R28 | no unapproved dependency/version/release/CI change | CLOSED (Phase 7; docs/plans-only diff) |
| R29 | Roadmap 076 final disposition is truthful | CLOSED (Phase 8; COMPLETE, evidence-backed) |
