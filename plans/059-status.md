# Plan 059 Status Ledger

Retrospective ledger. The delegation and generic-carrier decisions remain
current; later plans made the carrier internals private and added the narrow
parent-crate support layer.

## Baseline SHA

`55e0383d3495d10b5bdc435ab5295a3d3a13902f`

## Function Inventory

### Generic carrier mechanics → `src/stego/lsb.rs`

| Function | Current location | Destination |
|----------|-----------------|-------------|
| `stego_permutation` | `SteganographyProtector::stego_permutation` | `lsb::stego_permutation` (pub(crate)) |
| `stego_permutation_v2` | `SteganographyProtector::stego_permutation_v2` | `lsb::stego_permutation_v2` (pub(crate)) |
| `carrier_v2_slot_to_pixel_channel` | `SteganographyProtector::carrier_v2_slot_to_pixel_channel` | `lsb::carrier_v2_slot_to_pixel_channel` (pub(crate)) |
| `lsb_available_slots` | `SteganographyProtector::lsb_available_slots` | `lsb::lsb_available_slots` (pub(crate)) |
| `lsb_required_capacity_v2` | `SteganographyProtector::lsb_required_capacity_v2` | `lsb::lsb_required_capacity_v2` (pub(crate)) |
| `lsb_required_slots_legacy` | `SteganographyProtector::lsb_required_slots_legacy` | `lsb::lsb_required_slots_legacy` (pub(crate)) |
| `embed_bit_in_pixel` | `SteganographyProtector::embed_bit_in_pixel` | `lsb::embed_bit_in_pixel` (pub(crate)) |
| `bytes_to_bits` | `SteganographyProtector::bytes_to_bits` | `lsb::bytes_to_bits` (pub(crate)) |
| `bits_to_bytes` | `SteganographyProtector::bits_to_bytes` | `lsb::bits_to_bytes` (pub(crate)) |
| `embed_lsb` | `SteganographyProtector::embed_lsb` | `lsb::embed_lsb` (pub(crate)) |
| `extract_lsb` | `SteganographyProtector::extract_lsb` | `lsb::extract_lsb` (pub(crate)) |
| `extract_lsb_range` | `SteganographyProtector::extract_lsb_range` | `lsb::extract_lsb_range` (pub(crate)) |
| `embed_lsb_v2` | `SteganographyProtector::embed_lsb_v2` | `lsb::embed_lsb_v2` (pub(crate)) |
| `extract_lsb_v2` | `SteganographyProtector::extract_lsb_v2` | `lsb::extract_lsb_v2` (pub(crate)) |
| `embed_lsb_tiled` | `SteganographyProtector::embed_lsb_tiled` | `lsb::embed_lsb_tiled` (pub(crate)) |
| `crop_rgba` | `SteganographyProtector::crop_rgba` | `lsb::crop_rgba` (pub(crate)) |
| `blit_rgba` | `SteganographyProtector::blit_rgba` | `lsb::blit_rgba` (pub(crate)) |
| `embed_seed_lsb_fallback` | `SteganographyProtector::embed_seed_lsb_fallback` | `lsb::embed_seed_lsb_fallback` (pub(crate)) |
| `extract_seed_lsb_fallback` | `SteganographyProtector::extract_seed_lsb_fallback` | `lsb::extract_seed_lsb_fallback` (pub(crate)) |
| `splitmix64` | `splitmix64` (private) | `lsb::splitmix64` (pub(crate)) |
| `tile_seed` | `tile_seed` (pub) | `lsb::tile_seed` (pub) |
| `DEFAULT_TILE_SIZE` | `DEFAULT_TILE_SIZE` (pub const) | `lsb::DEFAULT_TILE_SIZE` (pub const) |
| `MIN_TILE_SIZE` | `MIN_TILE_SIZE` (pub const) | `lsb::MIN_TILE_SIZE` (pub const) |

### Generic carrier mechanics → `src/stego/jpeg.rs`

| Function | Current location | Destination |
|----------|-----------------|-------------|
| `dct_payload_capacity` | `SteganographyProtector::dct_payload_capacity` | `jpeg::dct_payload_capacity` (pub(crate)) |
| `reassemble_jpeg_with_qtables` | `SteganographyProtector::reassemble_jpeg_with_qtables` | `jpeg::reassemble_jpeg_with_qtables` (pub(crate)) |
| `embed_seed_hint` | (new) | `jpeg::embed_seed_hint` (pub(crate)) |
| `extract_seed_hint` | (new) | `jpeg::extract_seed_hint` (pub(crate)) |

### StegoEggo application logic (remains in `steganography.rs`)

| Function | Category |
|----------|----------|
| `generate_payload` | Payload generation |
| `generate_payload_for_context` | Payload generation |
| `parse_stego_payload` | Payload parsing |
| `parse_stego_payload_v1` | Payload parsing |
| `parse_stego_payload_v2` | Payload parsing |
| `parse_stego_payload_v3` | Payload parsing |
| `verify_payload_integrity` | Integrity check |
| `verify_checksum` | Integrity check |
| `compute_checksum` | Integrity check |
| `compute_payload_mac` | Integrity check |
| `compute_payload_mac_v3` | Integrity check |
| `verify_payload_mac` | Integrity check |
| `try_ecc_decode` | ECC decode |
| `classify_v3_prefix` | V3 classification |
| `classify_v3_probe` | V3 classification |
| `validate_v3_header` | V3 classification |
| `truncate_to_actual_payload` | V3 classification |
| `extract_embedded_seed` | Seed extraction |
| `classify_auth_failure` | V3 classification |
| `has_v3_magic` | V3 classification |
| `candidate_seed_matches` | Seed verification |
| `verify_embedded_seed_matches` | Seed verification |
| `lsb_pixels_needed` | Capacity query |
| `payload_bits_for_context` | Payload sizing |
| `lsb_slots_needed_for_bits` | Capacity query |
| `payload_within_limits` | Limit check |

### Orchestration (remains in `steganography.rs`)

| Function | Category |
|----------|----------|
| `apply_dct_stego_bytes` | JPEG orchestration |
| `apply_dct_stego_bytes_tiled` | JPEG orchestration |
| `apply_qtable_seed_bytes` | JPEG orchestration |
| `apply_lsb_minimal` | LSB orchestration |
| `apply_to_image_with_summary` | Main embed orchestration |
| `extract_payload` | Extraction orchestration |
| `extract_payload_with_key` | Extraction orchestration |
| `extract_payload_from_bytes_with_key` | Extraction orchestration |
| `extract_payload_with_seed_and_key` | Extraction orchestration |
| `extract_payload_with_seed` | Extraction orchestration |
| `extract_verified_dct_payload` | JPEG extraction |
| `extract_verified_dct_payload_from_coefficients` | JPEG extraction |
| `extract_with_redundancy` | LSB extraction orchestration |
| `extract_payload_at_seed_v2` | LSB extraction orchestration |
| `extract_payload_at_seed_legacy` | LSB extraction orchestration |
| `extract_lsb_tiled_candidates` | LSB tiled extraction |
| `extract_from_sub_image` | Tiled extraction |
| `probe_payload_from_prefix_v2` | V3 extraction |
| `probe_payload_from_prefix_legacy` | V3 extraction |
| `extract_f5_tiled_candidates` | JPEG tiled extraction |
| All `verify_extract_*` functions | Verification path |

## Implementation Commits

1. Added focused raw carrier tests (19 tests) proving arbitrary-payload behavior independently of StegoEggo payload-v3:
   - 7 LSB carrier tests: `raw_lsb_arbitrary_bytes_roundtrip`, `raw_lsb_binary_zero_ff_payload_roundtrip`, `raw_lsb_exact_capacity_outcome`, `raw_lsb_wrong_seed_not_equal`, `raw_lsb_legacy_scheme_fixture_roundtrip`, `raw_lsb_current_scheme_roundtrip`, `raw_lsb_has_no_rights_metadata_dependency`
   - 7 JPEG carrier tests: `raw_jpeg_arbitrary_bytes_roundtrip_supported_fixture`, `raw_jpeg_binary_zero_ff_payload_roundtrip`, `raw_jpeg_capacity_matches_supported_coefficients`, `raw_jpeg_wrong_seed_not_equal`, `raw_jpeg_container_segments_preserved`, `raw_jpeg_progressive_reports_unsupported_payload_embedding`, `qtable_seed_hint_roundtrip_does_not_imply_payload_success`
   - 5 application-adapter regression tests: `stegoeggo_png_v3_roundtrip_after_carrier_extraction`, `stegoeggo_webp_v3_roundtrip_after_carrier_extraction`, `stegoeggo_jpeg_v3_roundtrip_after_carrier_extraction`, `stegoeggo_hmac_wrong_key_classification_unchanged`, `stegoeggo_legacy_lsb_fixture_still_extracts`

## Focused Test Commands

```bash
cargo test --workspace --exclude stegoeggo-fuzz --all-features
```

## Dependency Boundary Disposition

`src/stego/lsb.rs` depends on:
- `image::RgbaImage`, `image::Rgba`, `image::GenericImageView`
- `crate::protected::constants::{SPLITMIX64_SEED, STEGO_OFFSET_SEED_1, STEGO_SPREAD_FACTOR, XORSHIFT_SEED_OFFSET}`
- `crate::types::EmbedOutcome`, `crate::types::EmbedPath`

`src/stego/lsb.rs` does NOT import:
- `ProtectionContext`, `ProtectionRequest`, `ResolvedProtectionPlan`, `ProtectionLevel`, `EvidenceProfile`
- `RightsPolicy`, `DmiValue`, `LegalMetadata`, `RightsNotice`
- `StegoPayload`, `NoticeVerification`, `VerificationReport`

`src/stego/jpeg.rs` depends on:
- `crate::jpeg_transcoder::{DctStegoF5, JpegTranscoder, JpegHeader}`
- `crate::types::EmbedOutcome`, `crate::types::EmbedPath`

`src/stego/jpeg.rs` does NOT import:
- Any application types listed in the boundary
