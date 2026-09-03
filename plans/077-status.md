# Plan 077 Status Ledger: Output-Domain Carrier Routing Correctness

Created before any product-source edit, per Plan 077 Phase 0.1.

## Baseline

- starting HEAD: `2d8d103f35bc1efef22118e053387df8c6dbc4db` (`plans: add stego pipeline and library closure plan`)
- audited planning baseline from plan text: `main` at `6feb52a90d9afdc0c922cdb219529524ad94c168`
- working tree status at ledger creation: clean (`git status --porcelain` empty, branch `main`)
- workspace versions: `stegoeggo 0.4.0`, `stegoeggo-stego 0.4.0`, `stegoeggo-cli 0.4.0`
- root -> carrier dependency declaration: `stegoeggo-stego = { path = "stegoeggo-stego", version = "=0.4.0", features = ["application-support"] }` (root `Cargo.toml`); CLI depends on root via `stegoeggo = { path = "..", version = "=0.4.0", features = ["iscc", "conformance", "parallel"] }`
- Roadmap 076 status: `Ready for implementation`, no implementation plan started (077/078/079/080 all still `Ready`)

## Requirement rows

| ID | Requirement | Status |
|----|-------------|--------|
| R01 | output-format carrier invariant encoded in tests | DONE (`tests/output_domain_routing.rs`, 17 tests) |
| R02 | JPEG->PNG BestEffort uses LSB carrier | DONE (verified + `EmbedPath::Lsb`) |
| R03 | JPEG->WebP BestEffort uses LSB carrier | DONE (verified + `EmbedPath::Lsb`) |
| R04 | JPEG->PNG Tiled uses tiled LSB carrier | DONE (verified + `EmbedPath::LsbTiled`) |
| R05 | JPEG->WebP Tiled uses tiled LSB carrier | DONE (verified + `EmbedPath::LsbTiled`) |
| R06 | PNG/WebP->JPEG uses DCT carrier | DONE (`EmbedPath::DctF5`, verified) |
| R07 | JPEG->JPEG encoded-byte fast path preserved | DONE (`EmbedPath::DctF5`, `format_transcoded=false`) |
| R08 | PNG/WebP same-format LSB behavior preserved | DONE (`EmbedPath::Lsb`, verified) |
| R09 | SeedOnly cross-format behavior matches final output domain | DONE (JPEG hint vs raster fallback matrix) |
| R10 | embed summary path matches actual final carrier | DONE (all matrix tests assert path) |
| R11 | Standard/Tiled executor duplication reduced without behavior drift | DONE (`execute_full_marker_and_metadata` + `apply_lsb_to_image_with_summary_from_plan`; removed `execute_stego_and_metadata*` + `embed_lsb_minimal_from_plan`) |
| R12 | legacy compatibility remains passing | DONE (workspace tests include `plan065_legacy_compat`, `plan026_gate1_2_3`; `check.sh` green) |
| R13 | focused request/public tests pass | DONE (`output_domain_routing` 17/17, plus full workspace suite) |
| R14 | full check.sh passes | DONE (`./scripts/check.sh` EXIT 0: fmt, clippy `-D warnings`, no-default-features check, workspace tests) |
| R15 | no unrelated API/dependency/release change | DONE (no public API, version, dependency, or CI change) |

## Implementation notes

- `src/protected/steganography/embed.rs`: replaced
  `apply_to_image_with_summary_from_plan()` (branched on `plan.input_format()`,
  including a JPEG re-encode/DCT branch inside the pixel helper) with
  `apply_lsb_to_image_with_summary_from_plan()` (raster-only; `EmbedPath` from
  `tile_size` only; PNG/WebP duplicate branches unified); deleted dead
  `embed_lsb_minimal_from_plan()` which had the same input-format dispatch.
  Legacy `apply_to_image_with_summary()` (context-based, used by deprecated
  pixel APIs) retained per plan 4.1.
- `src/lib.rs`: replaced `execute_stego_and_metadata()` +
  `execute_stego_and_metadata_tiled()` with single
  `execute_full_marker_and_metadata(img_bytes, plan, tile_size, ...)`;
  input/output formats obtained from the plan; carrier selected from
  `output_format` only; `(input, output)` used solely for JPEG-bytes reuse.
  `execute_seed_only_and_metadata()` left separate per plan 2.2 (already
  output-domain correct).
- Docs: `architecture/pipeline.md`, `architecture/protected-steganography.md`,
  `architecture/overview.md`, `architecture/resolve.md`,
  `architecture/protected-metadata-trap.md`, `architecture/jpeg-transcoder.md`
  updated to new executor/helper names and the explicit output-domain
  invariant; pruned stale `apply_bytes_pipeline` references and bumped stale
  `0.3.3`/`0.3` versions to `0.4.0`. `AGENTS.md` gotchas + architecture index +
  plan number (`081+`), `.skills/stegoeggo-conventions/SKILL.md` pitfall 17,
  `docs/formats.md` carrier-selection paragraph. README needs no change (no
  carrier-routing claims to correct).
- Tests: `tests/output_domain_routing.rs` — 9 required regression tests plus
  full BestEffort 3x3 table, Tiled 9-case table, and 4 SeedOnly direction
  tests (17 total).

## Phase 0 baseline observations

Regression file `tests/output_domain_routing.rs` (9 tests, textured 256x256,
JPEG quality 95, tile 64) run against unmodified baseline
(`2d8d103`, `cargo test -p stegoeggo --test output_domain_routing --all-features`):

- 5 passed: `png_to_png_best_effort_reports_lsb_path`,
  `webp_to_webp_best_effort_reports_lsb_path`,
  `png_to_jpeg_best_effort_reports_dct_path`,
  `webp_to_jpeg_best_effort_reports_dct_path`,
  `jpeg_to_jpeg_best_effort_reports_dct_path`.
- 4 failed as predicted by source inspection:
  - `jpeg_to_png_best_effort_verifies_lsb_output`: got `DctF5`, want `Lsb`
  - `jpeg_to_webp_best_effort_verifies_lsb_output`: got `DctF5`, want `Lsb`
  - `jpeg_to_png_tiled_verifies_lsb_output`: got `DctF5Tiled`, want `LsbTiled`
  - `jpeg_to_webp_tiled_verifies_lsb_output`: got `DctF5Tiled`, want `LsbTiled`

Root cause confirmed: `SteganographyProtector::apply_to_image_with_summary_from_plan`
in `src/protected/steganography/embed.rs:171` selects carrier from
`plan.input_format()`. The `execute_stego_and_metadata*` raster branch decodes
JPEG input to pixels, re-encodes to transient JPEG for DCT, decodes back to
pixels, then encodes to final PNG/WebP — the DCT payload is tied to the
transient representation. `R01` now has its failing matrix; `R02-R05` fail,
`R06-R08` pass at baseline.

Focused test file: `tests/output_domain_routing.rs`
Focused commands:

```bash
cargo test -p stegoeggo --test output_domain_routing --all-features
cargo test -p stegoeggo --test request_api --all-features
cargo test -p stegoeggo --test cross_format_semantics --all-features
cargo test -p stegoeggo --all-features stego
cargo test -p stegoeggo --all-features tiled
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```
