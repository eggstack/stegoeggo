# Plan 085 Status

Status: IMPLEMENTED

Baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`

## Required evidence

- [x] all PNG/JPEG/WebP structural walkers inventoried
- [x] canonical traversal selected per format
- [x] resource accounting consumes shared traversal/observer facts
- [x] duplicate `lib.rs` structural walker removed
- [x] malformed-input and boundary tests added
- [x] `ResourceUsage` compatibility checked
- [x] relevant fuzz targets build/smoke-run where available
- [x] repository-wide walker re-audit completed
- [x] `./scripts/check.sh` passes

## Traversal ownership map

Accounting owner: `src/container_walk.rs::observe_container_work()` (new, `pub(crate)`).
Dispatched per `ImageOutputFormat` from `src/pipeline.rs` executors
(`execute_metadata_only`, `execute_full_marker_and_metadata`,
`execute_seed_only_and_metadata`). `src/lib.rs` contains no container parser.

| Format | Accounting walk | Canonical strict parsers (separate domain) | Detection/extraction walkers |
|--------|----------------|--------------------------------------------|------------------------------|
| PNG | `container_walk::observe_png_work` — signature check, `while pos+12<=len`, BE `chunk_len`, break at `IEND`, `checked_add(12)`, `observe_png_chunk(total)`, `observe_metadata_field` for `tEXt`/`iTXt` | `metadata_trap/png.rs::inject_text_chunks_png` (IHDR/IEND validation, `ContainerLimitExceeded`/`ImageTruncated` errors) | `png_has_stego_metadata`, `collect_stego_owned_png_keys`, `extract_seed_from_png`, `strip_stego_owned_png`, `notice_verification::{extract_png_notice, extract_xmp_dmi_from_png_with_limits}` |
| JPEG | `container_walk::observe_jpeg_work` — SOI check, skip non-`0xFF` bytes, stop at `D9`/`DA`, skip `0x00` stuffing, `checked_add(2+seg_len)`, `observe_jpeg_segment`, `observe_metadata_field` for `E1`/`ED`/`FE` | `metadata_trap/jpeg.rs::inject_text_chunks_jpeg` (SOS pre-scan, strict `ImageTruncated` errors); carrier `JpegHeader::parse_with_limits` + `analyze_structure_checked` for DCT | `jpeg_has_stego_metadata`, `collect_stego_owned_jpeg_keys`, `extract_seed_from_jpeg`, `strip_stego_owned_jpeg`, `notice_verification::{extract_jpeg_notice, extract_xmp_dmi_from_jpeg_with_limits}` |
| WebP | `container_walk::observe_webp_work` — RIFF/WEBP check, LE `chunk_size`, `checked_add` padding, partial `observe_webp_chunk(len-pos)` on truncated tail | `webp_container::parse_webp` (exact RIFF-size match, VP8X/duplication validation) + `validate_webp_output`, used by `inject_text_chunks_webp_from_notice` | `webp_has_stego_metadata`, `collect_stego_owned_webp_keys`, `extract_seed_from_webp`, `strip_stego_owned_webp`, `notice_verification::{extract_webp_notice, extract_xmp_dmi_from_webp_with_limits}` |

Test helpers (not production): `src/bin/stegoeggo-conformance.rs`
`extract_xmp_from_{png,jpeg,webp}`, `src/conformance.rs::detect_format`.

## Bounds / error semantics

- All container arithmetic uses `checked_add`; overflow breaks leniently (accounting/detection) or returns `Error::Metadata`/`ImageTruncated` (strict injection/strip).
- Accounting is lenient on malformed input: bad magic returns `Ok(())`; truncated headers/segments break silently with partial counts; only `budget.check_limits()` can fail (`ContainerLimitExceeded` / `MetadataLimitExceeded`).
- No unbounded allocation: all walks are streaming/index-based over slices; `observe_container_work` allocates nothing.
- No public API added: `container_walk` is `pub(crate)`; `ResourceUsage` field values preserved (chunk/segment counts, `metadata_fields_extracted` per `tEXt`/`iTXt`/JPEG-metadata-marker, `metadata_bytes_copied`).
- Hardened during this plan (same lenient disposition, no behavior change on valid inputs): `png::extract_seed_from_png`, `jpeg::{jpeg_has_stego_metadata, extract_seed_from_jpeg, extract_seed_from_iptc, iptc_has_stego_properties, jpeg_payload_has_stego_properties(_static), collect_stego_owned_jpeg_keys, inject_text_chunks_jpeg pre-scan, png pre-scan}`, `webp::{extract_seed_from_webp, strip_stego_owned_webp, webp_has_stego_metadata, collect_stego_owned_webp_keys}`, and all six `notice_verification` extractors.

## Tests

- Unit (`src/container_walk.rs`, 16 tests): PNG/JPEG/WebP valid counts, `tEXt`/`iTXt` fields, IEND/SOS stop, truncated headers, oversized declared lengths (`u32::MAX` PNG, `u16::MAX` JPEG segment, `u32::MAX` WebP), odd WebP padding, truncated WebP tail partial observe, per-format limits at/beyond threshold, `max_metadata_field_bytes` trigger.
- Integration (`tests/container_accounting.rs`, 6 tests): end-to-end `ResourceUsage` container counts for real PNG/JPEG/WebP, plus limit-at-threshold-pass / below-threshold-fail via `process_request_bytes_with_report`.
- `ResourceUsage` compatibility: counts and `metadata_bytes_copied` preserved; JPEG `seg_len.saturating_sub(2)` and WebP chunk-only (no metadata-field) semantics unchanged.

## Fuzz disposition

- `cargo check --manifest-path fuzz/Cargo.toml` passes; `./scripts/check_fuzz_sync.sh` reports 12 targets synchronized.
- Smoke: `cargo +nightly fuzz run png_metadata -- -max_total_time=10` — 194108 runs, no crash.

## Final commit

`621769247c9ae5193afe6f117e25c4fa603ec245` — plan 085: container observer and resource-accounting reuse.

## Verification notes

- `./scripts/check.sh` passes locally; remote CI passed on rerun.
- First CI run hit a pre-existing flake unrelated to this plan:
  `tests/request_aux_convergence.rs::sync_variants_agree_jpeg` compares
  `process_request_bytes` vs `process_request_bytes_with_warnings` byte-for-byte,
  but `generate_structured_com_marker` (`src/protected/metadata_trap/notice.rs`)
  embeds `SystemTime::now()` seconds, so back-to-back calls straddling a second
  boundary differ. Reproduces on demand by looping the single test
  (failed on iteration 5 locally). No file touched by this plan generates that
  marker; left as a known issue for a future plan.
