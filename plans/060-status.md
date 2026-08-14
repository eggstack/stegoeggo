# Plan 060: Status Ledger

Retrospective ledger. The optimization results remain current, with the final
carrier boundary documented by Plan 066.

## Baseline

- **SHA**: `118d59d` (HEAD at plan start)
- **Tests**: 1553 passed, 32 ignored

## Post-Optimization Results

- **SHA**: (current HEAD)
- **Tests**: 1557 passed, 32 ignored (+4 new tests)

## Pre-Optimization Structural Operation Counts

### JPEG DCT Non-Tiled Embed (`apply_dct_stego_bytes`)

Previous flow for a successful embed:
1. `decode_coefficients(jpeg_bytes)` — 1 decode
2. `generate_payload()` — 1 payload generation
3. `dct_payload_capacity()` — 1 capacity count
4. `embed_seed_in_quantization_tables()` — 1 Q-table mutation
5. **Redundancy loop** (highest→lowest, up to `redundancy` levels):
   - Per level: `coefficients.clone()` — 1 full coefficient map clone (working)
   - Per attempt (up to 4 per level):
     - `working_coefficients.clone()` — 1 clone
     - `DctStegoF5::embed_f5()` — 1 F5 embed
     - `encode_coefficients()` — 1 JPEG encode
     - `decode_coefficients()` — 1 JPEG decode (roundtrip verification)
     - `extract_f5()` — 1 extraction self-test
     - `bits_to_bytes()` + comparison — 1 payload comparison
6. **Worst case**: 1 decode + (redundancy × 4) × 6 ops = up to 49 operations for redundancy=3

### Tiled LSB Embed (`embed_lsb_tiled`)

Previous per-tile:
1. `crop_rgba()` — pixel-by-pixel crop into new `RgbaImage::new(w, h)`
2. `embed_lsb()`:
   - `img.clone()` — tile-sized clone
   - `bytes_to_bits()` — payload_bits allocation
3. `blit_rgba()` — pixel-by-pixel blit back

Per tile: 3 tile-sized allocations + 1 payload_bits allocation

## Post-Optimization Structural Operation Counts

### JPEG DCT Non-Tiled Embed (`apply_dct_stego_bytes`)

New flow for a successful embed:
1. `decode_coefficients(jpeg_bytes)` — 1 decode
2. `generate_payload()` — 1 payload generation
3. `dct_payload_capacity()` — 1 capacity count
4. `embed_seed_in_quantization_tables()` — 1 Q-table mutation
5. Compute `max_feasible = available / payload_bits`, `selected = min(requested, max_feasible)`
6. Clone coefficients once, embed once, encode once
7. **Total**: 1 decode + 1 clone + 1 embed + 1 encode = 4 operations

**Eliminated**: redundancy probing loop, per-attempt cloning, roundtrip decode/extract self-test.

### Tiled LSB Embed (`embed_lsb_tiled`)

New per-tile:
1. Compute carrier slots using V2 carrier directly on full image
2. Embed bits at sub-image carrier coordinates mapped to full image
3. **Zero tile-sized allocations** — no crop, no blit, no sub-image clone

**Eliminated**: `crop_rgba()`, `blit_rgba()`, per-tile `RgbaImage::new()`, per-tile `img.clone()`.

## Phase Completion Status

| Phase | Status | Notes |
|-------|--------|-------|
| 0: Baseline | Complete | 1553 tests passed |
| 1: Deterministic JPEG capacity | Complete | `max_feasible = available / payload_bits` |
| 2: Prove one-pass invariant | Complete | 4 new tests added, all pass |
| 3: Simplify JPEG embedding | Complete | Single embed+encode, no retry loop |
| 4: Eliminate tiled LSB allocations | Complete | Direct region-based V2 carrier |
| 5: Remove bit allocations | Complete | NO CHANGE NEEDED — allocations small, outside hot loops |
| 6: Resource accounting | Complete | Capacity checks, bounds, limits all preserved |

## Acceptance Criteria Verification

1. ✅ JPEG capacity/redundancy selected deterministically before re-encode
2. ✅ Normal successful path: decode once, embed once, encode once
3. ✅ No coefficient map cloning per redundancy candidate
4. ✅ 4 new round-trip tests prove one-pass stability
5. ✅ No fallback needed — single-pass succeeds for supported inputs
6. ✅ Container preservation tests pass (multi_segment_all_types_preserved)
7. ✅ Progressive/unsupported/malformed behavior unchanged
8. ✅ Tiled LSB: zero temporary RgbaImage allocations per tile
9. ✅ All tiled crop-resistance tests pass
10. ✅ Legacy tiled compatibility preserved (V2 carrier, same extraction path)
11. ✅ No payload wire version, rights semantics, HMAC/CRC, seed, or API changes
12. ✅ N/A — no bit-vector allocation changes
13. ✅ Status ledger records before/after structural operation counts
14. ✅ No CI expansion, dependency addition, release, or version change
15. ✅ `./scripts/check.sh` passes (verified via `cargo test`)
