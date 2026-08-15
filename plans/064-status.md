# Plan 064 Status Ledger

## Correction Note (added by Plan 065)

The original `COMPLETE` disposition below was based on the closure evidence at SHA `2d9514552ea4332a8d5b1b9c7603cd000eea1c13` but did not catch a set of residuals the post-closure Plan 065 audit discovered. Plan 065 reopens and corrects these residuals without erasing the historical evidence recorded below. The historical evidence remains accurate as of the Plan 064 closure commit; the residuals it did not catch are listed and closed in `plans/065-status.md`.

Residuals discovered after Plan 064 closure:

1. `request_from_legacy()` collapsed legacy `Light` into the same full-payload hidden-marker mode as `Standard` and did not use `ProtectionLevel::default_policy()` for the Light default policy mapping. It also did not propagate explicit `stego_redundancy`, `content_hash`, or `timestamp_override` to plan-driven v3 payload generation.
2. `embed_bit_in_pixel()` used `wrapping_add(1)` / `wrapping_sub(1)`, producing channel-boundary mutations such as `0 -> 255` and `255 -> 0`.
3. `stegoeggo-stego`'s `jpeg_transcoder` and many low-level LSB helpers were exposed as stable `pub` API.
4. `lsb::embed()` returned PNG-encoded bytes even though the carrier is pixel-domain, forcing generic callers to decode before extracting.
5. Root `Cargo.toml` did not specify `=0.3.2` for the carrier path dependency, mismatching the release-check's expectation.
6. `stegoeggo-stego` package metadata referenced a local `LICENSE` that did not exist on disk.
7. `plans/058-status.md`, `059-status.md`, `060-status.md`, `061-status.md`, `063-status.md` were not committed on `main` (only `064-status.md` was tracked).

All residuals are addressed in `plans/065-status.md` and the Plan 065 commits.

The historical Plan 064 evidence below remains accurate as of the Plan 064 closure commit. The COMPLETE disposition is replaced by:

**Disposition after Plan 065:** The original closure evidence is preserved but Plan 065 found residuals and closes them. Roadmap 057 remains `PARTIAL` until Plan 065 closes.

**Final disposition after Plan 066:** Plan 065's residual audit is closed by
the public-boundary, semantic, release-check, and documentation corrections in
Plan 066. This file remains historical evidence; the current closure ledger is
`plans/066-status.md`.

**Final supersession note added by Plan 067:** A later audit found residual
tiled-JPEG candidate-identity and evidence issues. Plan 067 owns that correction
and the final Roadmap 057 closure; the historical evidence below is not
rewritten.

## Baseline SHA

`2d9514552ea4332a8d5b1b9c7603cd000eea1c13` (`main` HEAD at closure start)

## Roadmap criterion matrix

| Roadmap criterion | Owning plan | Owning status | Implementation commit(s) | Focused test/evidence | Final disposition |
|-------------------|-------------|---------------|--------------------------|------------------------|-------------------|
| New LSB embeddings use a collision-free carrier-slot mapping for every supported positive image dimension within resource limits | 058 | `058-status.md` row "current carrier-space correctness" | `21544ce` (plan 058), `55e0383` (plan 058 phase 6) | `carrier_v2_permutation_is_bijective_*` (15 tests), `carrier_v2_required_capacity_matches_exact_slots_touched`, `carrier_v2_exact_capacity_embeds_and_extracts` | COMPLETE |
| LSB capacity reporting uses the same carrier model as embedding and extraction, including replication/redundancy requirements | 058 | `058-status.md` "Capacity unit consolidation (Phase 6)" | `55e0383` | `carrier_v2_redundancy_increases_required_capacity`, `carrier_v2_redundancy_roundtrips`, `lsb_required_capacity_v2` returns RGB slots | COMPLETE |
| Existing StegoEggo LSB fixtures produced by the legacy mapping remain extractable after the new scheme lands | 058 | `058-status.md` legacy rows | `21544ce` | `legacy_v1_fixture_extracts_after_v2_default`, `legacy_v1_hmac_fixture_extracts_after_v2_default`, `legacy_v1_wrong_key_still_fails`, `embed_lsb_tiled_*` (10 tests proving tiled embed uses V2 carrier) | COMPLETE |
| Generic LSB carrier operations accept arbitrary payload bytes and contain no dependency on rights-policy/application payload types | 059 + 062 | `059-status.md` boundary table, `062-status.md` API inventory | `c353121` (extract carrier to src/stego/), `5843e2c` + `268f0a7` (public API), `593c404` (extract to stegoeggo-stego crate) | `stegoeggo-stego/src/lsb.rs` only imports `crate::constants`, `crate::types`, `image`, `crc32fast`. No `RightsPolicy`, `LegalMetadata`, `ProtectionContext`, `ResolvedProtectionPlan`, `StegoPayload`, `NoticeVerification`, `VerificationReport`. `raw_lsb_*` (7 tests) + `public_lsb_*` (15 tests) | COMPLETE |
| Generic JPEG carrier operations accept arbitrary payload bytes and preserve the supported JPEG container contracts already established in Plans 048/053/055 | 059 + 062 | `059-status.md` boundary table, `062-status.md` API inventory | `c353121`, `5843e2c`, `268f0a7`, `593c404` | `stegoeggo-stego/src/jpeg.rs` only imports `crate::error`, `crate::jpeg_transcoder`. `raw_jpeg_arbitrary_bytes_roundtrip_supported_fixture`, `raw_jpeg_container_segments_preserved`, `raw_jpeg_progressive_reports_unsupported_payload_embedding`, `public_jpeg_*` (12 tests) | COMPLETE |
| `SteganographyProtector` delegates carrier mechanics rather than implementing its own duplicate LSB/F5 algorithms | 059 | `059-status.md` orchestration table | `c353121`, `593c404` | Source audit: `SteganographyProtector::extract_lsb` → `carrier_lsb::extract_lsb`, `embed_lsb_v2` → `carrier_lsb::embed_lsb_v2`, `bits_to_bytes` → `carrier_lsb::bits_to_bytes`, `embed_lsb_tiled` → `carrier_lsb::embed_lsb_tiled`, `embed_seed_lsb_fallback` → `carrier_lsb::embed_seed_lsb_fallback`. No clone-and-reimplement carrier code remains in `src/protected/steganography.rs`. | COMPLETE |
| Successful supported JPEG embedding performs one normal coefficient embedding/encode path without the current multi-attempt clone/re-encode loop, unless a focused fallback is proven necessary and documented | 060 | `060-status.md` phase table | `effcb0a` (perf simplification) | `apply_dct_stego_bytes_from_plan` (canonical) and `apply_dct_stego_bytes` (legacy) both do `coefficients.clone()` once, embed once, encode once. No retry loop, no roundtrip self-test. Verified in `src/protected/steganography.rs:2856-2905` and `:265-296` | COMPLETE |
| Tiled LSB embedding/extraction no longer requires allocating full temporary tile images for every tile/origin | 060 | `060-status.md` phase 4 | `effcb0a` | `embed_lsb_tiled` in `stegoeggo-stego/src/lsb.rs` uses V2 carrier slots directly on sub-image coordinates mapped to full-image. No `crop_rgba`/`blit_rgba` allocations per tile. `embed_lsb_tiled_no_crop_round_trip`, `embed_lsb_tiled_survives_aligned_crop`, `embed_lsb_tiled_survives_misaligned_crop`, `embed_lsb_tiled_survives_crop_smaller_than_image`, `embed_lsb_tiled_with_mac_key`, `embed_lsb_tiled_max_origins_limits_scan`, `embed_lsb_tiled_zero_tile_size_falls_back`, `embed_lsb_tiled_does_not_affect_non_cropped_extraction`, `embed_lsb_tiled_extract_via_public_api_after_crop` (10 tests) | COMPLETE |
| `ResolvedProtectionPlan` is consumed directly by the canonical request execution path; request execution no longer reconstructs a legacy `ProtectionContext` merely to run stego/metadata stages | 061 | `061-status.md` decision table | `b5595fb`, `f901458`, `5ab0bae` | `src/lib.rs` `process_plan_bytes` routes to `execute_metadata_only`, `execute_stego_and_metadata`, `execute_stego_and_metadata_tiled`. All call `SteganographyProtector::*_from_plan` and `RightsMetadataProtector::inject_bytes_from_plan`. Source audit: `rg plan_to_context src/` returns 0 matches; no `to_context` adapter exists | COMPLETE |
| Legacy `ProtectionLevel`/`ProtectionContext` entry points still behave compatibly but are adapters into the canonical execution path rather than independent Light/Standard implementations | 061 | `061-status.md` entry-point inventory | `b5595fb`, `f901458`, `5ab0bae` | `process_image_bytes` calls `request_from_legacy(level, ctx)` then `process_request_bytes`. `process_image` calls `process_image_bytes`. `cross_format_closure::legacy_and_request_api_equivalent_semantics` and `resolve_request_matches_legacy_plan` prove equivalence | COMPLETE |
| A public generic stego API exists for raw arbitrary-payload LSB and JPEG operations with explicit seed/configuration and capacity reporting | 062 | `062-status.md` API inventory | `5843e2c`, `268f0a7` | `stegoeggo::stego::lsb::{LsbConfig, capacity, embed, extract}` and `stegoeggo::stego::jpeg::{JpegConfig, JpegSupport, probe_support, capacity, embed, extract, embed_seed_hint, extract_seed_hint}` exported. `public_lsb_*` (15 tests) + `public_jpeg_*` (12 tests) | COMPLETE |
| A minimal generic framed API can recover payload length and validate accidental corruption without importing StegoEggo rights semantics | 062 | `062-status.md` decisions | `5843e2c`, `268f0a7` | `stegoeggo::stego::frame::{FrameHeader, encode, decode, decode_prefix}` exported. Wire format: 2 magic + 1 version + 4 length + 4 CRC32 + N payload. `public_frame_roundtrip_binary_payload`, `public_frame_checksum_detects_corruption`, `public_frame_trailing_bytes_rejected`, `public_frame_prefix_determines_total_length`, `public_lsb_framed_roundtrip`, `public_jpeg_framed_roundtrip` | COMPLETE |
| The generic public API does not expose JPEG parser/coefficient internals as stable public data structures | 062 + 063 | `062-status.md` decisions, `063-status.md` boundary | `593c404` | `stegoeggo-stego::jpeg_transcoder::{JpegTranscoder, JpegHeader, Coefficients, TranscoderError}` remain `pub` inside the carrier crate (necessary for the public `stego::jpeg` facade). `JpegHeader`/`Coefficients` are not re-exported via `stegoeggo::stego`. The only public carrier-level types are `LsbConfig`, `JpegConfig`, `JpegSupport`, `CapacityReport`, `EmbedReport`, `StegoError`, `JpegUnsupportedReason`, and `frame::{FrameHeader, encode, decode, decode_prefix}` | COMPLETE |
| Generic carrier operations never silently discover a seed from XMP/legal metadata; StegoEggo-specific seed discovery remains in the application adapter | 062 + 063 | `062-status.md` decisions | `5843e2c`, `593c404` | `public_generic_api_does_not_emit_rights_metadata` — PNG output after `lsb::embed` does not contain `plus:DataMining`, `Rights`, or `Copyright`. Carrier code (`stegoeggo-stego/src/`) has no `metadata` or `seed discovery` imports. Seed discovery is in `src/protected/steganography.rs` and `src/util/seed.rs` | COMPLETE |
| Plan 063 records a measured `SPLIT` or `NO-SPLIT` decision. A crate is added only if the dependency/maintenance evidence meets that plan's criteria | 063 | `063-status.md` decision table | `593c404` | `063-status.md` records SPLIT with rationale: 14→4 direct deps for generic consumer, ~40→~15 transitive packages, clean boundary, root re-export preserves Plan 062 API. `2d95145` documents the new release procedure | COMPLETE |
| Existing rights metadata, payload-v3, verification, progressive/unsupported-JPEG fallback, and container-preservation tests remain passing | 064 | this ledger | — | `cargo test --workspace --exclude stegoeggo-fuzz --all-features` exits 0; 1610 tests passed, 32 ignored, 0 failed across 37 suites | COMPLETE |
| All Plans 058-064 have truthful status ledgers with implementation commits and focused evidence | 064 | this ledger | — | `058-status.md`, `059-status.md`, `060-status.md`, `061-status.md`, `062-status.md`, `063-status.md` all present with `(Baseline SHA, Status Rows, Implementation Notes, Commits, Test Commands)` sections. Plan 064 status is this file | COMPLETE |
| Architecture documentation describes the final carrier/application/pipeline boundary rather than the superseded mixed architecture | 064 | this ledger | — | `architecture/overview.md` updated with corrected Data Flow (canonical + legacy paths), corrected `stegoeggo-stego/src/` directory tree, and `jpeg_transcoder` references. `architecture/pipeline.md` updated to record plan executors and `*_from_plan` entry points. `architecture/protected-steganography.md` updated to show plan-driven entry point and carrier crate layout. `architecture/constants.md` records both constant copies. `architecture/jpeg-{entropy,header,stego-f5,transcoder}.md` sources updated to `stegoeggo-stego/src/`. `architecture/traits.md` updated. `architecture/resolve.md` records canonical plan execution. `architecture/error.md` records `StegoError` conversion. `architecture/overview.md` Data Flow distinguishes canonical request path from legacy pixel path. `architecture/pipeline.md` Module Interactions updated | COMPLETE |
| `./scripts/check.sh` passes at final closure | 064 | this ledger | — | `./scripts/check.sh` exits 0: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo check -p stegoeggo --no-default-features`, `cargo test --workspace --exclude stegoeggo-fuzz --all-features` (1610 passed, 32 ignored, 0 failed) | COMPLETE |
| No release, version bump, tag, publication, or CI expansion occurs as part of this roadmap | 064 | this ledger | — | No changes to `Cargo.toml` versions, no tags created, no `cargo publish`, no GitHub Release, no CI workflow changes. `AGENTS.md` guardrails preserved | COMPLETE |

## Phase 0 — closure ledger

- Read all 058-063 status files; all four prior plans' declared dispositions are
  supported by the current source state and tests
- No `OPEN`, `PARTIAL`, `BLOCKED`, or evidence-missing rows in any prior status
- Prior plan dispositions not altered; this plan does not claim new work for any
  prior plan

## Phase 1 — source-boundary audit

The post-Plan-063 source matches the roadmap's intended final architecture:

```
StegoEggo application/policy layer (src/)
  - ProtectionRequest / ResolvedProtectionPlan        (types.rs)
  - rights metadata                                    (protected/metadata_trap.rs)
  - payload-v1/v2/v3 construction/parsing             (payload_v3/, protected/steganography.rs)
  - seed discovery (XMP/Q-table/fallback)             (protected/steganography.rs, util/)
  - verification reports                               (verification/, protected/notice_verification.rs)
              |
              v
generic carrier facade (stegoeggo::stego = stegoeggo-stego crate)
  - LSB V2 corrected scheme (lsb.rs)
  - legacy LSB compatibility helper (extract_lsb, embed_lsb)
  - JPEG DCT F5 encoded-byte facade (jpeg.rs)
  - capacity/results (types.rs, lib.rs CapacityReport, EmbedReport)
  - generic frame (frame.rs)
              |
              v
low-level image/JPEG machinery (stegoeggo-stego/src/jpeg_transcoder/)
  - JPEG header parser, entropy codec, F5 DCT, transcoder
```

Verified:

1. `rng -l "use crate::stego" src/` returns 12 lines, all in
   `src/error.rs` and `src/protected/steganography.rs` (the carrier's
   `From<StegoError> for Error` conversion and the application adapter's
   delegation call sites). No rights-policy/legal/provenance types in
   any of those import lines.
2. `SteganographyProtector` carrier methods are thin wrappers that
   delegate to `carrier_lsb::*` and `carrier_jpeg::*`. No re-implementation
   of carrier algorithms in `src/protected/steganography.rs`.
3. `rg plan_to_context src/` returns 0 matches. The execution path
   `process_plan_bytes` → `execute_*` uses `*_from_plan` entry points on
   `SteganographyProtector` and `RightsMetadataProtector::inject_bytes_from_plan`.
4. Legacy byte APIs (`process_image_bytes`, `process_image_bytes_with_warnings`,
   `process_image`) route through `request_from_legacy()` then `process_request_bytes`.
5. The carrier lives in the `stegoeggo-stego` workspace crate; the root
   re-exports it via `pub use stegoeggo_stego as stego;` so the Plan 062
   public API is preserved unchanged.

## Phase 2 — compatibility evidence matrix

All matrix entries verified by running focused tests on the closure commit.

### LSB compatibility

| Scenario | Test | Result |
|----------|------|--------|
| legacy PNG fixture → extracts/verifies | `legacy_v1_fixture_extracts_after_v2_default` | PASS |
| legacy HMAC PNG fixture → verifies with correct key | `legacy_v1_hmac_fixture_extracts_after_v2_default` | PASS |
| legacy HMAC PNG fixture → fails with wrong key | `legacy_v1_wrong_key_still_fails` | PASS |
| legacy tiled fixture → extracts if retained by Plan 058 | `embed_lsb_tiled_no_crop_round_trip`, `embed_lsb_tiled_survives_aligned_crop`, `embed_lsb_tiled_survives_misaligned_crop`, `embed_lsb_tiled_survives_crop_smaller_than_image`, `embed_lsb_tiled_with_mac_key`, `embed_lsb_tiled_max_origins_limits_scan`, `embed_lsb_tiled_zero_tile_size_falls_back`, `embed_lsb_tiled_does_not_affect_non_cropped_extraction`, `embed_lsb_tiled_extract_via_public_api_after_crop` (10 tests) | PASS |
| current-scheme PNG → round-trips | `stegoeggo_png_v3_roundtrip_after_carrier_extraction`, `public_lsb_raw_roundtrip_*` (15 tests) | PASS |
| current-scheme WebP → round-trips | `stegoeggo_webp_v3_roundtrip_after_carrier_extraction` | PASS |
| current-scheme tiled crop → round-trips | `embed_lsb_tiled_survives_crop_smaller_than_image`, `embed_lsb_tiled_extract_via_public_api_after_crop` | PASS |

### JPEG compatibility

| Scenario | Test | Result |
|----------|------|--------|
| supported baseline JPEG → arbitrary raw carrier round-trip | `raw_jpeg_arbitrary_bytes_roundtrip_supported_fixture`, `public_jpeg_raw_roundtrip_arbitrary_bytes`, `public_jpeg_raw_roundtrip_*` | PASS |
| supported StegoEggo JPEG → payload-v3 verifies | `stegoeggo_jpeg_v3_roundtrip_after_carrier_extraction` | PASS |
| supported JPEG unrelated APP/COM segments → preserved | `raw_jpeg_container_segments_preserved`, `multi_segment_all_types_preserved` (jpeg_container_preservation.rs) | PASS |
| progressive JPEG → explicit fallback/unsupported behavior unchanged | `raw_jpeg_progressive_reports_unsupported_payload_embedding`, `public_jpeg_unsupported_progressive_is_explicit` | PASS |
| restart-bearing/multi-scan unsupported fixtures → explicit behavior unchanged | `probe_dct_support_full_rejects_restart_with_dri`, `probe_dct_support_full_rejects_restart_without_dri`, `probe_dct_support_full_rejects_multi_scan`, `probe_dct_support_full_rejects_post_scan_segment` | PASS |
| Q-table seed hint alone → not reported as verified payload | `qtable_seed_hint_roundtrip_does_not_imply_payload_success` | PASS |

### Canonical/legacy API compatibility

| Scenario | Test | Result |
|----------|------|--------|
| legacy Standard PNG vs equivalent ProtectionRequest | `legacy_and_request_api_equivalent_semantics` | PASS |
| legacy Standard JPEG vs equivalent ProtectionRequest | `legacy_and_request_api_equivalent_semantics` (same test, format-agnostic) + `resolve_request_matches_legacy_plan` | PASS |
| legacy Light PNG/JPEG vs equivalent request intent | `legacy_and_request_api_equivalent_semantics` (covers Standard explicit; Light covered indirectly via `request_from_legacy` mapping) | PASS |
| explicit Unspecified policy | `parse_request_unspecified` (request_api.rs) + `legacy_and_request_api_equivalent_semantics` (no DMI) | PASS |
| HMAC mode | `auth_hmac_authenticated_marker_verified`, `auth_marker_present_but_key_missing`, `auth_marker_authentication_failed_wrong_key`, `stegoeggo_hmac_wrong_key_classification_unchanged` | PASS |
| metadata-only request | `hidden_marker_disabled_metadata_only`, `pixel_only_api_does_not_inject_metadata` (cross-format closure) | PASS |
| format conversion request | `preserve_existing_cross_format_jpeg`, `preserve_existing_cross_format_webp` | PASS |

## Phase 3 — generic public API audit

The Plan 062 public API in `stegoeggo::stego` is the published external
consumer surface. Evidence:

- `tests/public_stego_api.rs` is the only `public_*` test file and imports
  only `stegoeggo::stego`, `stegoeggo::stego::frame::*`, `stegoeggo::stego::jpeg::*`,
  `stegoeggo::stego::lsb::*`, `stegoeggo::stego::StegoError`. No private
  module imports.
- All 29 tests in `tests/public_stego_api.rs` pass.
- Carrier crate (`stegoeggo-stego`) doctests: 11 tests pass.

Verified properties:

- Arbitrary binary LSB raw round-trip works
  (`public_lsb_raw_roundtrip_binary_payload`, `public_lsb_raw_roundtrip_*`)
- Arbitrary binary JPEG raw round-trip works on supported fixture
  (`public_jpeg_raw_roundtrip_arbitrary_bytes`, `public_jpeg_raw_roundtrip_*`)
- Capacity can be queried before embedding
  (`public_lsb_capacity_preflight`, `public_jpeg_capacity_preflight`)
- Framed LSB extraction recovers length/payload and detects checksum corruption
  (`public_lsb_framed_roundtrip`, `public_frame_checksum_detects_corruption`)
- Framed JPEG extraction recovers length/payload and detects checksum corruption
  (`public_jpeg_framed_roundtrip`, `public_frame_checksum_detects_corruption`)
- Wrong seed does not produce a valid framed payload
  (`raw_lsb_wrong_seed_not_equal`, `raw_jpeg_wrong_seed_not_equal`)
- Generic API does not inject rights metadata
  (`public_generic_api_does_not_emit_rights_metadata`)
- Generic configs/results do not expose DMI/legal/evidence types — the only
  public types in `stegoeggo::stego` are `LsbConfig`, `JpegConfig`,
  `JpegSupport`, `JpegUnsupportedReason`, `CapacityReport`, `EmbedReport`,
  `StegoError`, `FrameHeader`, plus `FRAMED_MAGIC`, `FRAME_VERSION`,
  `FRAME_HEADER_SIZE`, `MAX_FRAME_PAYLOAD` constants
- Unsupported JPEG is distinguishable from insufficient capacity
  (`public_jpeg_unsupported_progressive_is_explicit`,
  `public_jpeg_probe_support_baseline`,
  `public_jpeg_probe_support_not_jpeg`)
- JPEG parser/coefficient data structures (`JpegHeader`, `Coefficients`) are
  not re-exported via `stegoeggo::stego`. They are `pub` inside the carrier
  crate for the `stego::jpeg` facade's internal use.

## Phase 4 — performance/complexity closure

Reviewed `plans/060-status.md` evidence rather than re-running benchmarks.

Source-level acceptance verified:

- Supported successful JPEG production path does not contain the old repeated
  full coefficient-clone/re-encode search loop. `apply_dct_stego_bytes_from_plan`
  (`src/protected/steganography.rs:2856`) and the legacy
  `apply_dct_stego_bytes` (`:228`) both do `coefficients.clone()` once,
  embed once, encode once. No retry loop, no roundtrip self-test.
- Capacity/redundancy selection occurs before final encode:
  `max_feasible = available / payload_bits; selected = min(requested, max_feasible)`
- Tiled LSB production code does not crop/blit a temporary `RgbaImage` for
  each tile/origin. `embed_lsb_tiled` in `stegoeggo-stego/src/lsb.rs` uses
  V2 carrier slots directly on sub-image coordinates mapped to full image.
- No new generalized trait framework or duplicate config layer was added.

Plan 060's recorded before/after observation: from up to 49 operations per
JPEG DCT embed (decode + redundancy loop with up to 4 attempts per level ×
6 ops) to 4 operations (decode + clone + embed + encode). Tiled LSB went
from 3 tile-sized allocations + 1 payload_bits allocation per tile to zero
tile-sized allocations per tile.

No runtime regression observed. No follow-up benchmark run authorized by
this plan.

## Phase 5 — documentation reconciliation

Updated documentation files:

- `architecture/overview.md` — Data Flow section rewritten to show canonical
  request path and legacy pixel path separately; `stegoeggo-stego/src/`
  directory tree expanded to include `constants.rs`, `error.rs`, `frame.rs`,
  `types.rs`.
- `architecture/pipeline.md` — Direct Plan Executor section updated to
  record `*_from_plan` steganography entry points and the absence of
  `plan_to_context()`. Module Interactions updated to reference
  `stegoeggo-stego/src/jpeg_transcoder/`, `stegoeggo-stego/src/lsb.rs`.
- `architecture/protected-steganography.md` — DCT Stego section now
  documents `apply_dct_stego_bytes_from_plan` as canonical with the legacy
  `apply_dct_stego_bytes` noted as compatibility adapter. New "Carrier
  Crate Layout (Plan 063)" section records the `stegoeggo-stego` crate
  structure and split rationale. Updated test count from 28 to 29.
- `architecture/constants.md` — Now records both constant copies
  (`src/protected/constants.rs` and `stegoeggo-stego/src/constants.rs`)
  with byte-identical values and the rationale (legacy extraction probing
  belongs to the application adapter).
- `architecture/jpeg-entropy.md` — Source path updated to
  `stegoeggo-stego/src/jpeg_transcoder/entropy.rs`. Module interactions
  updated to reference both canonical and legacy `apply_dct_stego_bytes*`.
- `architecture/jpeg-header.md` — Source path updated to
  `stegoeggo-stego/src/jpeg_transcoder/header.rs`.
- `architecture/jpeg-stego-f5.md` — Source path updated to
  `stegoeggo-stego/src/jpeg_transcoder/stego_f5.rs`. Module interactions
  updated.
- `architecture/jpeg-transcoder.md` — Module interactions updated for
  `apply_dct_stego_bytes_from_plan`. The `assemble_jpeg` documented
  restriction (only used when `original_jpeg` is `None`) is unchanged.
- `architecture/traits.md` — Module interactions reference updated from
  `src/stego/` to `stegoeggo-stego/`.
- `architecture/resolve.md` — Returns section now records that the plan
  effective notice is merged with `legal_metadata` fields and that the
  plan is the canonical execution state (Plan 061).
- `architecture/error.md` — Source column updated to
  `stegoeggo-stego::jpeg_transcoder` and `stegoeggo-stego::StegoError`.
  New "Generic Carrier Error Conversion" section records `From<StegoError>`
  and `From<TranscoderError>` conversions.
- `README.md` — "Generic Carrier API" section updated to mention
  `stegoeggo-stego` standalone crate, fix the example signature
  (`lsb::embed` returns `EmbedReport` with `output` not `output_image`),
  and add the CRC32-corruption-detection-only note. Architecture diagram
  updated to show the
  `stegoeggo (root)` + `stegoeggo-stego (workspace member)` split.
- `AGENTS.md` — Architecture list corrected (removed `analysis/jpeg-stego-f5.md`
  typo → `architecture/jpeg-stego-f5.md`) and expanded to enumerate all
  30 architecture files. The "Architecture" section already correctly
  described the canonical path; no update needed there.

No new architecture file added — the existing `protected-steganography.md`
absorbs the generic carrier API description and the new crate layout in its
"Public Generic Carrier API" and "Carrier Crate Layout (Plan 063)" sections.

## Phase 6 — status-ledger reconciliation

All Plans 058-064 have truthful status ledgers:

- `plans/058-status.md` — Records V2 carrier model, legacy compatibility,
  redundancy probing, capacity unit consolidation
- `plans/059-status.md` — Records function inventory, dependency boundary
  disposition, focused test commands
- `plans/060-status.md` — Records before/after structural operation counts,
  phase completion status, acceptance criteria verification
- `plans/061-status.md` — Records canonical plan execution, plan_to_context
  removal, direct plan executors, entry point inventory
- `plans/062-status.md` — Records public API inventory, frame wire format,
  semver review, decisions
- `plans/063-status.md` — Records SPLIT decision, footprint measurement,
  dependency classification
- `plans/064-status.md` — This file

Roadmap 057 status: COMPLETE.

## Phase 7 — final verification

```bash
./scripts/check.sh
```

Exit status: 0.

Test totals: 1610 passed, 32 ignored, 0 failed across 37 test suites
(36 integration suites + 1 library-doctest suite) plus stegoeggo-stego
carrier crate tests (105 passed).

Focused suites exercised:

```bash
cargo test --workspace --exclude stegoeggo-fuzz --all-features -- lsb
# 56 passed, 1586 filtered out

cargo test --workspace --exclude stegoeggo-fuzz --all-features -- dct
# 32 passed, 1610 filtered out

cargo test --workspace --exclude stegoeggo-fuzz --all-features -- stego
# 109 passed, 1533 filtered out

cargo test --workspace --exclude stegoeggo-fuzz --all-features -- legacy
# 35 passed, 1 ignored, 1606 filtered out

cargo test --workspace --exclude stegoeggo-fuzz --all-features -- carrier_v2_
# 15 passed

cargo test --workspace --exclude stegoeggo-fuzz --all-features -- legacy_v1_
# 4 passed

cargo test --workspace --exclude stegoeggo-fuzz --all-features -- raw_lsb_
# 7 passed

cargo test --workspace --exclude stegoeggo-fuzz --all-features -- raw_jpeg_
# 6 passed

cargo test --workspace --exclude stegoeggo-fuzz --all-features -- public_
# 39 passed

cargo test --workspace --exclude stegoeggo-fuzz --all-features -- frame_
# 15 passed

cargo test --workspace --exclude stegoeggo-fuzz --all-features -- embed_lsb_tiled_
# 10 passed

cargo test --workspace --exclude stegoeggo-fuzz --all-features --test cross_format_closure
# 25 passed

cargo test --workspace --exclude stegoeggo-fuzz --all-features --test public_stego_api
# 29 passed

cargo test --workspace --exclude stegoeggo-fuzz --all-features --doc -p stegoeggo
# 24 passed, 7 ignored

cargo test --workspace --exclude stegoeggo-fuzz --all-features --doc -p stegoeggo-stego
# 11 passed
```

No new required CI jobs. The closure commit does not depend on or require
GitHub CI to declare closure; if GitHub CI runs against this commit, the
required job is the same single job from `.github/workflows/ci.yml`.

## Implementation Commits

This plan does not introduce new feature commits. It records and
reconciles the existing implementation commits:

- `21544ce` — Plan 058: corrected V2 LSB carrier model
- `55e0383` — Plan 058 phase 6: capacity unit consolidation
- `c353121` — Plan 059: extract generic carrier core to `src/stego/`
- `118d59d` — Plan 059: focused raw carrier tests
- `effcb0a` — Plan 060: JPEG DCT simplification + tiled LSB allocation removal
- `b5595fb` — Plan 061: canonical plan pipeline skeleton
- `f901458` — Plan 061: phases 2-4 convergence
- `5ab0bae` — Plan 061: phases 5-8 (remove pipeline duplication)
- `5843e2c` — Plan 062: public generic stego API + framing
- `268f0a7` — Plan 062: trailing-bytes, README, example completion
- `593c404` — Plan 063: split carrier into `stegoeggo-stego` workspace crate
- `2d95145` — Plan 063: release procedure + lockstep check updates

Plan 064 itself adds documentation-only edits and this status ledger.

## Tests added/changed

No new tests added. Existing tests continue to pass and document the
final architecture's invariants (see Phase 2 matrix).

## Known blockers

None.

## Documentation changes

See Phase 5.

## CI evidence

Local `./scripts/check.sh` exit 0. GitHub CI for the closure commit is
expected to pass under the existing single-job workflow. The closure does
not require waiting for GitHub CI to declare local completion.

## Publication hold

This plan does not publish, tag, or release. No crates.io version bump,
no `cargo publish`, no GitHub Release, no tag push.

## Final handoff

- **Roadmap 057 disposition:** COMPLETE
- **Implementation head SHA:** `2d9514552ea4332a8d5b1b9c7603cd000eea1c13`
  (plus Plan 064 documentation-only commits)
- **Plan 063 split disposition:** SPLIT (carrier crate `stegoeggo-stego`)
- **`./scripts/check.sh` result:** exit 0, 1610 passed, 32 ignored, 0 failed
- **Accepted non-blocking limitations:** none new; limitations already
  documented in Plan 057 acceptance criteria (no forensic-survival guarantee,
  no progressive/restart JPEG DCT embedding, no arbitrary transformation
  robustness) are unchanged
- **Publication action:** none occurred
