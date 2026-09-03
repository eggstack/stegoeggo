# Plan 078 Status Ledger: Single-Decode and Tiled Allocation Optimization

Created before any product-source edit, per Plan 078 Phase 0.1.

## Baseline

- starting HEAD: `16858cebe1c2522cb8d1e6edc6d47f01fff01c7a`
  (`plan 077: output-domain carrier routing correctness`)
- audited planning baseline from plan text: `main` at
  `6feb52a90d9afdc0c922cdb219529524ad94c168`
- working tree status at ledger creation: clean
  (`git status --porcelain` empty, branch `main`)
- workspace versions: `stegoeggo 0.4.0`, `stegoeggo-stego 0.4.0`,
  `stegoeggo-cli 0.4.0`
- root -> carrier dependency declaration: `stegoeggo-stego = { path =
  "stegoeggo-stego", version = "=0.4.0", features = ["application-support"] }`
  (root `Cargo.toml:48`); CLI depends on root via `stegoeggo = { path = "..",
  version = "=0.4.0", features = ["iscc", "conformance", "parallel"] }`
- Plan 077 disposition: complete, committed as `16858ce` on `main`
  (output-domain carrier routing; `tests/output_domain_routing.rs`, 17 tests)
- carrier application-support feature wiring: `stegoeggo-stego` exposes hidden
  `application_support` module (`src/lib.rs:48-50`, `#[doc(hidden)]`,
  feature `application-support`); root enables it and aliases it as
  `carrier_support` (`src/protected/steganography/mod.rs:17`); stable facade
  is `lsb` / `jpeg` / `frame` / `error` only (re-exported via root
  `src/lib.rs:222-226` `pub mod stego`)

## Requirement rows

| ID | Requirement | Status |
|----|-------------|--------|
| ID | Requirement | Status |
|----|-------------|--------|
| R01 | non-JPEG preflight does not perform a discarded full pixel decode | DONE (preflight block deleted; `plan078_metadata_only_png_performs_no_pixel_decode` == 0) |
| R02 | raster marker path performs one pixel decode | DONE (`plan078_png_marker_performs_single_pixel_decode`, tiled request variant) |
| R03 | same-format JPEG marker path performs zero pixel decodes | DONE (unchanged fast path; pre-existing behavior, still covered by `jpeg_fast_path` + routing tests) |
| R04 | same-format JPEG marker path performs one coefficient decode | DONE (standard `jpeg::embed` decodes once; tiled now decodes once per R09 test) |
| R05 | tiled LSB in-place core exists | DONE (`lsb_internal::embed_lsb_tiled_in_place`) |
| R06 | tiled LSB insufficient-capacity path leaves mutable input unchanged | DONE (`tiled_in_place_leaves_carrier_unchanged_on_insufficient_capacity`) |
| R07 | tiled cloning path delegates to in-place core | DONE (`embed_lsb_tiled` is a clone-once wrapper; pixel-identity test) |
| R08 | parent tiled raster path avoids second full-image buffer | DONE (plan + legacy PNG/WebP branches mutate owned `rgba`; buffer-count derivation in Phase 5) |
| R09 | tiled JPEG embed performs one coefficient decode | DONE (`tiled_jpeg_embed_decode_count_per_operation` == 1) |
| R10 | tiled JPEG production self-check does not re-decode encoded output | DONE (in-memory `jpeg_tiled_payload_matches_decoded`; fresh-decode roundtrip test retained) |
| R11 | standard JPEG application verification performs one coefficient decode | DONE (1 context per verification; stego + root counter tests) |
| R12 | standard+tiled JPEG verification can reuse one decoded carrier when both are attempted | DONE (shared context in `verify_extract_dct_with_seed`, `extract_verified_dct_payload`, payload-from-bytes JPEG branch; single-context tiled-fallback test) |
| R13 | JPEG candidate search coverage/order remains unchanged | DONE (redundancy 1..=10 order, prefix/full/legacy probing, tiled enumeration bodies verbatim in shared helpers) |
| R14 | malformed/wrong-seed/error precedence remains compatible | DONE (decode-failure equivalence argument; `plan078_malformed_input_still_fails_closed`; full suite green) |
| R15 | deterministic LSB/JPEG known-answer behavior remains passing | DONE (full workspace suite green via check.sh) |
| R16 | benchmark/decode/allocation evidence recorded | DONE (Phase 5 table + disposition) |
| R17 | public JPEG implementation structs remain private | DONE (`JpegSearchContext` fields private; no impl types in signatures) |
| R18 | focused carrier/root tests pass | DONE (stego lib 130+, root lib 599 green; full workspace run pending) |
| R19 | full check.sh passes | DONE (exit 0: fmt, clippy `-D warnings`, no-default-features check, workspace tests) |
| R20 | no API/dependency/release/CI scope expansion | DONE (no new deps; removed 2 dead `pub(crate)` fns; hidden `JpegSearchContext` + `tiled_lsb_embed_in_place` are parent-only support, no stable API change; no version/CI change; bench-only additions) |

## Phase 0 baseline observations (measured, not assumed)

Source locations verified against `16858ce` (see code-map below):

- Raster preflight (`src/lib.rs:1038-1074`): non-JPEG inputs run
  `ImageReader::into_dimensions()` (header-only) AND a full
  `load_image_from_bytes()` whose `DynamicImage` is used only for a
  `dimensions()` check and then discarded. Executors
  (`execute_full_marker_and_metadata`, `execute_seed_only_and_metadata`,
  `execute_metadata_only` cross-format) decode again. So a normal PNG/WebP
  marker operation currently performs **2 full pixel decodes**; same-format
  metadata-only PNG/WebP performs **1 wasted full decode**.
- JPEG preflight (`src/lib.rs:1023-1037`) is already header-only
  (`stego::jpeg::inspect`); JPEG->JPEG marker path performs **0 pixel
  decodes** (R03 already holds; R04 holds for standard embed via
  `jpeg::embed`, which decodes once).
- Standard application JPEG verification (`extract.rs:602-670`):
  `dct_candidates()` calls one-shot `carrier_support::jpeg_extract()` once
  per redundancy (1..=10), and each call fully entropy-decodes via
  `decode_supported_carrier()`. `dct_outcome_with_seed()` calls
  `dct_candidates()` for the V3 prefix (up to 10 decodes) and then again per
  prefix for the full payload (up to 10 more) or per legacy length (up to 20
  more). Normal V3 success path = **20 coefficient decodes** per seed;
  worst-case NotFound fan-out is an order of magnitude larger. The outer
  `verify_payload_from_bytes_outcome` JPEG branch can run the whole search
  twice (Q-table seed, then metadata seed).
- Tiled fallback (`verify_extract_dct_with_seed`, `extract.rs:672-720`)
  constructs a separate `TiledJpegSearch` (own decode) after the standard
  search, so standard+tiled currently costs standard decodes + 1.
- Tiled JPEG embed (`application_support.rs:147-294`): `jpeg_embed_tiled()`
  decodes once (line 159-161), encodes (line 222), then
  `jpeg_tiled_payload_matches()` re-decodes the encoded output (line 276)
  for the production self-check = **2 coefficient decodes + 1 encode**.
- Tiled LSB (`lsb_internal.rs:523-660`): writes directly into carrier
  coordinates (no per-tile image allocation) but starts from
  `let mut output = img.clone()` (line 540). The parent
  (`embed.rs:180-190`) additionally owns `rgba = img.to_rgba8()`, so a tiled
  raster operation holds **2 full-image buffers** beyond the decoded source.
- Existing test-only counters: `JPEG_COEFFICIENT_DECODE_COUNT`
  (`stegoeggo-stego/src/jpeg.rs:48-51`, incremented in
  `decode_supported_carrier`, asserted in `framed_extraction_*` tests) and
  `TILED_SEARCH_DECODE_COUNT`
  (`stegoeggo-stego/src/application_support.rs:10-28`, incremented in
  `decode_tiled_search_coefficients`, asserted in `tiled_jpeg_search_*`
  tests). Both prove single-decode behavior for the paths they cover.
  Neither covers the application verification fan-out, the tiled-embed
  re-decode, or raster pixel decodes.

### Code map (baseline `16858ce`)

- `process_plan_bytes` preflight: `src/lib.rs:1015-1119`
  (non-JPEG double decode at 1040-1053; JPEG header-only at 1023-1037)
- raster executors: `src/lib.rs:1250-1308` (`execute_full_marker_and_metadata`),
  `src/lib.rs:1311-1367` (`execute_seed_only_and_metadata`),
  `src/lib.rs:1121-1146` (`execute_metadata_only`)
- parent tiled LSB dispatch: `src/protected/steganography/embed.rs:165-199`
  (plan path), `src/protected/steganography/embed.rs:291-373` (legacy
  context path, PNG/WebP tiled branches)
- tiled LSB carrier: `stegoeggo-stego/src/lsb_internal.rs:523-660`
  (`embed_lsb_tiled`, clone at line 540); in-place standard core at
  `lsb_internal.rs:360-458` (`embed_lsb_v2_in_place`, no clone)
- application DCT search: `src/protected/steganography/extract.rs:602-670`
  (`dct_candidates`, `dct_outcome_with_seed`), `extract.rs:672-749`
  (`verify_extract_dct_with_seed`, `extract_verified_dct_payload`,
  `verify_extract_verified_dct`), `verify.rs:69-94`
  (`verify_payload_from_bytes_outcome` JPEG branch),
  `extract.rs:1286-1339` (`extract_payload_from_bytes_with_key` JPEG branch)
- generic single-decode precedent: `stegoeggo-stego/src/jpeg.rs:102-150`
  (`DecodedJpegCarrier`, `decode_supported_carrier`,
  `extract_from_decoded`), `jpeg.rs:773-831` (`extract_framed` decodes once,
  probes redundancies from retained state)
- tiled JPEG embed + self-check: `stegoeggo-stego/src/application_support.rs:147-294`
- tiled JPEG single-decode precedent: `TiledJpegSearch`
  (`application_support.rs:305-440`, decodes once in `new()`)
- transcoder equivalence note: `JpegTranscoder::decode_coefficients` is
  `decode_coefficients_with_probe` with `Unsupported` mapped to `Err`
  (`stegoeggo-stego/src/jpeg_transcoder/mod.rs:191-223`). Both the standard
  probe path and the tiled search path therefore degrade to `NotFound` on
  any decode failure, which is what makes sharing one decoded carrier
  behavior-preserving (verified by R13/R14 tests, not by assumption).
- benches: `benches/bench.rs` (13 groups; `lsb_clone_vs_in_place` covers
  standard LSB clone vs in-place at 1024/4096; `tiled_embed`/`tiled_extract`
  are JPEG-only; no PNG tiled bench; benches reach the carrier only through
  root `stego::` stable re-exports, so carrier-level tiled benching is out
  of scope until Plan 079 decides the public tiled API)

### Measurement plan (Phase 0.2)

Test-only seams (no production telemetry):

1. `stegoeggo-stego`: new `#[cfg(test)]` counter on the tiled-embed decode
   sites (`jpeg_embed_tiled` + `jpeg_tiled_payload_matches`); baseline test
   asserts 2, post-fix test asserts 1. `decode_supported_carrier` counter
   already proves one `jpeg::extract` == one decode; baseline fan-out
   (10 per `dct_candidates`, 20 per successful `dct_outcome`) follows by
   construction and is recorded here.
2. Root crate: new `#[cfg(test)]` counter around `load_image_from_bytes`
   (narrow seam in `src/util/image.rs`); baseline tests assert 2 decodes
   for a PNG marker operation and 1 wasted decode for same-format
   metadata-only; post-fix asserts 1 and 0. (A `decode_coefficients`
   counter cannot be observed from root tests because dependency crates are
   built without `cfg(test)`; JPEG decode-count evidence therefore rests on
   (a) stego-crate counter tests proving one `JpegSearchContext` == one
   decode, plus (b) a root `#[cfg(test)]` context-creation counter proving
   one context per verification operation. This split is recorded as an
   explicit limitation, not a gap.)
3. Allocation baseline for tiled LSB at 1024x1024 RGBA: 4 MiB per full-image
   buffer; current path holds decoded source + `to_rgba8()` clone + tiled
   clone. Post-fix path holds decoded source + one owned RGBA. Recorded as
   buffer-count evidence plus Criterion wall-clock (`tiled_lsb_request`
   pipeline group, `jpeg_standard_verify` group); no CI bench gates added.

## Implementation notes

(Updated as work lands; every entry cites files/lines and the R-rows it closes.)

### Phase 0 instrumentation (test-only seams, no behavior change)

- `stegoeggo-stego/src/application_support.rs:11-39` — new `#[cfg(test)]`
  `TILED_EMBED_DECODE_COUNT` incremented at both tiled-embed decode sites
  (`jpeg_embed_tiled`, `jpeg_tiled_payload_matches`); baseline test
  `tiled_jpeg_embed_decode_count_per_operation` asserts **2** (passes).
- `stegoeggo-stego/src/jpeg.rs` tests — `one_shot_extract_decodes_once_per_call`
  proves one `jpeg::extract` == one coefficient decode (10 calls -> count 10,
  passes). Combined with `dct_candidates()` calling `jpeg_extract` once per
  redundancy (1..=10, `extract.rs:609-614`), the baseline application fan-out
  is 10 decodes per candidate probe and 20 per successful V3
  `dct_outcome_with_seed` (prefix + full), by construction.
- `src/util/image.rs:207-229` — new `#[cfg(test)]` `LOAD_IMAGE_DECODE_COUNT`
  seam in `load_image_from_bytes` with `pub(crate)` reset/get helpers.
- `src/lib.rs` tests — `plan078_baseline_png_marker_pixel_decode_count`
  asserts **2** full pixel decodes for a PNG hidden-marker request (passes);
  `plan078_baseline_metadata_only_png_pixel_decode_count` asserts **1**
  wasted decode for same-format metadata-only (passes).
- Limitation recorded: dependency crates build without `cfg(test)` for root
  tests, so JPEG coefficient decodes cannot be counted directly from root
  tests. JPEG evidence therefore chains (a) stego-crate counter tests (one
  context/search == one decode) with (b) a root `#[cfg(test)]`
  context-creation counter (one context per verification operation, Phase 2).

### Phase 1 — discarded preflight decode removed (R01, R02)

- `src/lib.rs` `process_plan_bytes`: deleted the eager
  `load_image_from_bytes()` block whose `DynamicImage` was used only for a
  `dimensions()` check and discarded. Retained the header-only
  `ImageReader::into_dimensions()` gate enforcing
  `ResourceLimits::check_dimensions` + `max_dimension` before any decode;
  executors still defensively re-check dimensions after their (single)
  decode and fail closed on malformed input.
- The removed early error string (`metadata-only processing requires a valid
  image`) had no test dependents; cross-format undecodable input now errors
  from the executor's decode (`Error::ImageDecode`, same variant), and
  same-format metadata-only on garbage errors from container injection as
  before (preflight contributed nothing there).
- Tests (`src/lib.rs`): `plan078_png_marker_performs_single_pixel_decode`
  (1 decode), `plan078_metadata_only_png_performs_no_pixel_decode` (0
  decodes), `plan078_malformed_input_still_fails_closed` (garbage still
  errors on both marker and metadata-only paths).

### Phase 2 — one private decoded-JPEG search context (R11, R12, R13, R14, R17)

- `stegoeggo-stego/src/jpeg.rs`: `DecodedJpegCarrier` is now `pub(crate)`
  and retains the parsed `JpegHeader` alongside coefficients/capacity, with
  `pub(crate)` `extract_raw` / `capacity_for` / `header` / `coefficients` /
  `available_capacity` accessors. `decode_supported_carrier` is `pub(crate)`.
  No coefficient/header type crosses the crate boundary (R17).
- `stegoeggo-stego/src/application_support.rs`: new hidden
  `JpegSearchContext` (all fields private; signatures mention only
  `StegoResult`, `CapacityReport`, `Vec<u8>`, tile keys) with `new` (exactly
  one coefficient decode), `extract`, `capacity`, `available_capacity`,
  `tiled_prefix_candidates`, `tiled_extract_candidate`. The tiled
  enumeration/extraction bodies were factored into shared free functions
  used verbatim by both `TiledJpegSearch` (unchanged one-shot behavior) and
  the context — one decoded-carrier representation serves standard and tiled
  search. Sharing is behavior-preserving because `decode_coefficients` is
  `decode_coefficients_with_probe` with `Unsupported` mapped to `Err`
  (`jpeg_transcoder/mod.rs:191-223`): every decode failure degrades both
  standard and tiled paths to `NotFound`, so a failed context creation
  returns `NotFound` with identical observable outcomes.
- Root `src/protected/steganography/extract.rs`: `dct_candidates` and
  `dct_outcome_with_seed` now take `&JpegSearchContext` (redundancy order
  1..=10 and prefix/full/legacy probing unchanged, R13);
  `verify_extract_dct_with_seed` creates one context for standard search +
  tiled fallback (`verify_extract_f5_tiled_with_context`); the now-unused
  `verify_extract_f5_tiled` bytes variant was removed.
  `extract_verified_dct_payload` and the `extract_payload_from_bytes_with_key`
  JPEG branch share one context across standard + tiled attempts.
  `verify_payload_from_bytes_outcome` still performs up to two verification
  operations (Q-table seed, then metadata seed) — one decode each, matching
  the plan's "1 per verification operation/search context" target.
- Evidence: stego test
  `jpeg_search_context_decodes_once_for_standard_and_tiled_probes` (20
  standard extracts + capacities + full tiled candidate scan == 1 decode);
  root tests `plan078_standard_jpeg_verification_uses_single_search_context`
  and `plan078_tiled_fallback_jpeg_verification_uses_single_search_context`
  (each verification == exactly 1 context creation). Root creations are
  counted by a `#[cfg(test)]` seam around `new_jpeg_search_context`.

### Phase 3 — tiled JPEG embed re-decode removed (R09, R10)

- `stegoeggo-stego/src/application_support.rs` `jpeg_embed_tiled`: the
  production self-check now extracts the first successful tile from the
  already-mutated in-memory coefficients
  (`jpeg_tiled_payload_matches_decoded`) instead of re-decoding the encoded
  output. `SkippedCapacity` fallback semantics on invariant failure are
  unchanged. The bytes-decoding `jpeg_tiled_payload_matches` and
  `jpeg_tiled_outcome_after_verification` helpers are `#[cfg(test)]`-gated
  and retained as roundtrip evidence.
- Evidence: `tiled_jpeg_embed_decode_count_per_operation` now asserts **1**
  (was 2 at baseline); new
  `tiled_jpeg_embed_output_roundtrips_through_fresh_decode` proves encoded
  output still verifies through a fresh decode plus `TiledJpegSearch`
  recovery. Tiled embed is now 1 coefficient decode + 1 encode.

### Phase 4 — in-place tiled LSB core (R05, R06, R07, R08)

- `stegoeggo-stego/src/lsb_internal.rs`: new `embed_lsb_tiled_in_place`
  (`&mut RgbaImage` -> `InPlaceEmbedReport`, per-tile redundancy 1). Phase A
  scans tile geometry/capacity with no mutation (insufficient capacity —
  geometry skip, bit-length overflow, no fitting tile, mid-scan slot
  overflow — returns before any pixel is touched, R06); Phase B mutates only
  planned tiles using the exact corrected-V2 permutation/seed derivation.
  Defensive per-slot reports reproduce the cloning version's running totals
  exactly. `embed_lsb_tiled` is now a clone-once wrapper delegating to the
  core (R07).
- `application_support::tiled_lsb_embed_in_place` added (hidden,
  parent-only); the cloning `tiled_lsb_embed` is unchanged for compat.
- Root `src/protected/steganography/embed.rs`: plan-based
  `apply_lsb_to_image_with_summary_from_plan` and both legacy
  context-based PNG/WebP tiled branches now mutate the already-owned `rgba`
  buffer via `embed_lsb_tiled_in_place` (R08); summaries map to
  `EmbedPath::LsbTiled` with identical status/capacity fields. The
  now-unused protector-level cloning `embed_lsb_tiled` was removed; its 8
  test call sites now use `carrier_support::tiled_lsb_embed` directly.
- Evidence: `tiled_in_place_matches_cloning_embed` (pixel-identical),
  `tiled_in_place_report_matches_cloning_capacities` (identical
  required/available figures), 256x256 tiled request verifies with exactly
  1 pixel decode (`plan078_tiled_png_request_verifies_with_single_decode`).

### Final cleanup (post-Phase 5, pre-commit)

- Tiled candidate enumeration/extraction shared helpers were regrouped from
  two 8-argument free functions into a private `TiledGrid` view struct
  (clippy `too_many_arguments` under `-D warnings`); `TiledJpegSearch` and
  `JpegSearchContext` both delegate to it with byte-identical enumeration
  bodies. One `let...else`-vs-`?` clippy finding fixed the same way.
- Removed the now-unused one-shot `application_support::jpeg_extract`
  (duplicated stable `jpeg::extract`; zero workspace callers after the
  `JpegSearchContext` migration) and the unused protector-level cloning
  `embed_lsb_tiled` (8 test call sites moved to
  `carrier_support::tiled_lsb_embed`). No stable public API changed.
- `benches/bench.rs`: new `jpeg_standard_verify` (textured JPEG verify) and
  `tiled_lsb_request` (textured PNG tiled protect) groups; `tiled_embed` /
  `tiled_extract` switched from solid-black to textured fixtures (black
  JPEGs carry ~zero non-zero AC coefficients, so those groups previously
  timed the skipped-capacity path) with `sample_size(10)` and a `Verified`
  setup assertion on the extract side. Bench-only changes; no CI gates.
- Docs: `architecture/protected-steganography.md` (in-memory tiled-JPEG
  self-check, shared in-place tiled core, single-decode verification flow,
  Plan 078 design-decision item), `architecture/pipeline.md` (header-only
  preflight wording), `AGENTS.md` (three gotchas + adapter description),
  `.skills/stegoeggo-conventions/SKILL.md` (pitfalls 14/18/19). README and
  user `docs/` need no change (no affected claims).

## Phase 5 evidence

Deterministic decode/allocation counts come from the unit tests cited in
Phases 0-4. Wall-clock figures below are local Criterion evidence only
(`cargo bench --bench bench -- --quick <group>`, one filter per invocation;
no CI bench gates added). Before/after compares `16858ce` (pre-078 baseline,
via a detached worktree running the same bench file) against the post-fix
tree on identical textured inputs. Setups assert `Verified` where noted so
each group measures the intended success path.

| group (success path) | 256 before | 256 after | 1024 before | 1024 after |
|---|---|---|---|---|
| `jpeg_standard_verify` (standard JPEG verify) | ~85.7 ms | ~8.5 ms | ~1345 ms | ~137 ms |
| `tiled_extract` (standard miss + tiled hit) | ~803 ms | ~177 ms | ~10.09 s | ~2.6 s |
| `tiled_embed` (textured JPEG tiled protect) | ~16.2 ms | ~9.2 ms | ~1729 ms | ~1099 ms |
| `tiled_lsb_request` (textured PNG tiled protect) | ~4.97 ms | ~4.15 ms | ~59.8 ms | ~52.5 ms |

Notes and limitations:

- `--quick` implies higher variance than default Criterion runs; the
  direction is consistent across all four groups and all sizes, and the
  ~10x standard-verify win matches the deterministic 20->1 decode-count
  reduction (the 1024 `tiled_extract` baseline bounds are identical at
  10.092 s, suggesting a measurement ceiling — recorded as measured).
- The `tiled_embed`/`tiled_extract` groups were switched from solid-black to
  textured fixtures in this plan: black JPEGs carry ~zero non-zero AC
  coefficients, so the old groups measured the skipped-capacity path rather
  than the F5 tiled embed/verify they name.
- Allocation evidence is buffer-count derivation, not a sampled allocator
  trace: one RGBA buffer is `w*h*4` bytes (256 KiB at 256x256, 4 MiB at
  1024x1024). The tiled raster path held decoded source + `to_rgba8()`
  clone + tiled clone (3 buffers); it now holds decoded source + one owned
  RGBA (2 buffers). No new allocator instrumentation was added; the
  existing `allocations_512x512` group is untouched.
- No cross-format raster bench was added: the JPEG->PNG win (2 pixel
  decodes -> 1) is covered deterministically by the `LOAD_IMAGE_DECODE_COUNT`
  unit tests, and existing `bytes_processing`/`format_preservation` groups
  already time those routes.

### Prepared-public-API disposition for Plan 079

**`PRIVATE-REUSE-SUFFICIENT`**

Rationale: the only demonstrated multi-operation reuse is the application
verification search (arbitrary prefixes/lengths/redundancies plus tiled
fallback), which is now served by the hidden `JpegSearchContext` with all
codec types private. Generic callers performing repeated framed
capacity/extraction on one JPEG are already served by one-shot
`jpeg::extract_framed` (single retained decode per call); no concrete
external workflow in this repo performs multiple one-shot operations on one
JPEG where retained state would materially reduce work beyond that. No
public prepared/reusable JPEG type is justified by the recorded evidence.

## Focused commands

```bash
cargo test -p stegoeggo-stego --all-features
cargo test -p stegoeggo-stego --doc
cargo test -p stegoeggo --test public_stego_api --all-features
cargo test -p stegoeggo --all-features jpeg
cargo test -p stegoeggo --all-features tiled
cargo test -p stegoeggo --all-features verification
cargo test --workspace --exclude stegoeggo-fuzz --all-features
cargo bench --bench bench -- --quick jpeg_standard_verify tiled_lsb_request
./scripts/check.sh
```
