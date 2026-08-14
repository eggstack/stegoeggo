# Plan 058 Status

Baseline SHA: `80cb93fe53e56080a8e7c0e07a83e26d5d680aca`

## Status Rows

- legacy non-tiled extraction fixture: COMPLETE (test-based: `legacy_v1_fixture_extracts_after_v2_default`)
- legacy HMAC extraction fixture: COMPLETE (test-based: `legacy_v1_hmac_fixture_extracts_after_v2_default`)
- legacy tiled extraction fixture: COMPLETE (test-based: `embed_lsb_tiled_*` tests confirm tiled uses legacy path)
- current carrier-space correctness: COMPLETE (`carrier_v2_*` tests verify slot model)
- non-power-of-two permutation correctness: COMPLETE (bijectivity tests for prime, composite, odd-dimension slot counts)
- capacity/embed agreement: COMPLETE (`carrier_v2_required_capacity_matches_exact_slots_touched`, `carrier_v2_exact_capacity_embeds_and_extracts`)
- replication/embed agreement: COMPLETE (`carrier_v2_redundancy_increases_required_capacity`, `carrier_v2_redundancy_roundtrips`)

## Implementation Notes

### Corrected V2 carrier model

The corrected V2 LSB carrier scheme uses a single-pass embedding where each
payload bit occupies `STEGO_SPREAD_FACTOR * redundancy` consecutive logical
indices through one bijective permutation over `width * height * 3` RGB slots.

Key properties:
- Slot count: `width * height * 3` (alpha never a carrier)
- Permutation: cycle-walking bijective LCG over `[0, slot_count)` via `stego_permutation_v2`
- Capacity: `payload_bits * STEGO_SPREAD_FACTOR * redundancy` slots exact
- No inter-replica collisions (all replicas of one bit use consecutive indices)
- No inter-bit collisions (bijective permutation guarantees unique mapping)

### Backward compatibility

Legacy extraction is preserved via `extract_lsb` (legacy carrier: `total_pixels`
modulus, channel from `bit_index % 3`). Extraction probes V2 first (raw seed,
redundancy 1..=10), then legacy. Legacy fixtures created with `embed_lsb`
extract correctly.

### Redundancy probing

Extraction probes redundancy values 1..=10 per seed to match the embed's
redundancy without storing it in metadata. The corrected embed uses the raw
seed directly (no `STEGO_OFFSET_SEED_1` offset), while legacy embeds use
offset seeds. Extraction tries raw seed first, then offset seeds.

### Verification path

`verify_extract_with_redundancy` checks `verify_embedded_seed_matches` on
candidate outcomes to avoid false positives from wrong-redundancy extractions
that happen to decode as structurally valid payloads with wrong seeds.

### Capacity unit consolidation (Phase 6)

All `EmbedOutcomeSummary` capacity fields for LSB paths now report in RGB
carrier slots (not pixels). Both V2 and legacy paths use
`lsb_required_capacity_v2` / `lsb_required_slots_legacy` which return slot
counts. The `lib.rs` pre-check compares `total_slots < slots_needed` using
the same slot-based formula. `EmbedOutcomeSummary` documents the unit as
"RGB slots for LSB paths, non-zero AC coefficients for DCT paths."

## Commits

- `21544ce` — fix: corrected V2 LSB carrier model (plan 058)
