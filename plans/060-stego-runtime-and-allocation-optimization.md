# Plan 060: Stego Runtime and Allocation Optimization

Status: Ready for implementation

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

Depends on: Plans 058-059 complete.

Audited planning baseline: `main` at `e60b801fda493ddd5e744f5de2a12d7b9489c078`

Authoritative implementation ledger to create before source edits: `plans/060-status.md`

---

## 1. Purpose

Reduce avoidable work in the extracted stego carrier layer without changing supported formats, carrier semantics, compatibility behavior, or fail-closed parsing.

The two primary targets are:

1. supported JPEG DCT embedding, which currently may clone coefficient maps and repeatedly encode/decode/extract while searching for a successful redundancy level; and
2. tiled LSB embedding/extraction, which currently materializes temporary cropped `RgbaImage` tiles and blits them back.

Secondary targets such as temporary bit vectors are authorized only when the resulting code is simpler or measurably cheaper.

This plan is not permission to weaken verification or container preservation for benchmark wins.

---

## 2. Phase 0 — record a truthful pre-optimization baseline

Before product edits:

1. Create `plans/060-status.md` and record the actual baseline SHA.
2. Identify the exact successful supported-JPEG path in the new `stego::jpeg` carrier facade.
3. Record the number of full coefficient clones, JPEG encodes, JPEG decodes, and payload extraction validations performed by that path in the current source for a successful embed.
4. Record the tiled LSB allocation pattern: number/size of temporary tiles allocated for representative images.
5. Run focused correctness tests before changing implementation.
6. If existing Criterion benches cover these paths, record representative results. If not, add only narrow benchmarks that directly measure the two owned hot paths.

Benchmark numbers are informational. Acceptance is based on structural work elimination plus correctness, not a brittle percentage threshold.

Suggested representative fixtures:

```text
PNG 1024x1024 non-tiled
PNG 2048x2048 tiled 64x64
baseline JPEG 1024x1024 4:2:0 supported DCT
baseline JPEG 2048x2048 supported DCT
progressive JPEG fallback fixture
low-capacity JPEG fixture
```

---

## 3. Phase 1 — make JPEG capacity selection deterministic before mutation

The supported JPEG carrier should determine the maximum feasible redundancy from decoded coefficient capacity before cloning/mutating coefficients.

Required model:

```text
payload_bits = payload.len() * 8
available = usable AC coefficient count
max_feasible_redundancy = available / payload_bits
selected = min(requested_redundancy, max_feasible_redundancy)
```

Apply existing redundancy bounds. If `selected == 0`, return the established skipped-capacity outcome while preserving any separately required Q-table seed-hint behavior.

The capacity helper and F5 embed implementation must agree on the same carrier eligibility rule. If capacity counts `|coef| >= 2`, embedding must consume exactly that stable set after any required coefficient normalization.

Required tests:

```text
jpeg_selected_redundancy_is_highest_feasible
jpeg_capacity_zero_skips_without_payload_embedding
jpeg_capacity_exact_for_selected_redundancy_roundtrips
jpeg_requested_redundancy_is_not_exceeded
```

Do not probe redundancy levels by repeatedly encoding complete JPEGs when capacity already determines the feasible value.

---

## 4. Phase 2 — prove the one-pass supported JPEG invariant

Before deleting round-trip retry logic, strengthen focused tests around the invariant that supported DCT embedding survives the carrier's own entropy re-encode.

Required proof matrix should include:

- 4:4:4, 4:2:2, and 4:2:0 supported baseline JPEG fixtures where available;
- positive and negative AC coefficients near the minimum carrier magnitude;
- coefficients near representable bounds;
- redundancy 1 and at least one higher redundancy;
- arbitrary binary payload bytes, not only StegoEggo v3 payloads;
- preserved APP2/APP13/APP14/COM/unknown APP fixtures already used by container-correctness tests.

The test sequence should explicitly perform:

```text
decode coefficients once
embed once
encode preserving original container once
decode emitted JPEG for test verification
extract payload from emitted coefficients
assert exact payload equality
```

The test-side verification decode is allowed. The goal is to remove repeated production self-validation, not to stop testing the encoder invariant.

If this matrix reveals a real class of supported inputs where one-pass output is unstable, do not delete all fallback logic. Narrow the supported subset or retain one focused fallback and document the reason in the status ledger.

---

## 5. Phase 3 — simplify production JPEG embedding

Preferred successful production path after the invariant is proven:

```text
parse/probe supported structure
-> decode coefficients once
-> compute payload/capacity once
-> select feasible redundancy once
-> modify Q-table seed hint once if enabled
-> embed payload into one mutable coefficient map
-> encode preserving original JPEG once
-> return outcome
```

Requirements:

- no loop that clones the full coefficient map for multiple redundancy guesses;
- no repeated full JPEG encode/decode cycle on the normal successful path;
- no extraction self-test on every successful production embed unless Phase 2 proves it is required;
- unsupported/progressive behavior remains explicit and unchanged;
- malformed entropy remains a hard error;
- preserving encoder remains mandatory when an original JPEG is available;
- capacity skip reports the actual selected/required/available values consistently.

If a test-only self-check helper remains useful, keep it under `#[cfg(test)]` or benchmark/test infrastructure rather than in the hot path.

---

## 6. Phase 4 — eliminate tiled LSB crop/blit image allocations

Replace temporary tile-image extraction with direct region-based carrier traversal.

Preferred shape:

```rust
fn embed_region(
    image: &mut RgbaImage,
    region: Rect,
    payload: &[u8],
    config: &LsbConfig,
) -> ...

fn extract_region(
    image: &RgbaImage,
    region: Rect,
    payload_len: usize,
    config: &LsbConfig,
) -> ...
```

`Rect` may be a small private struct or explicit `(x, y, width, height)` arguments. Do not introduce a general geometry framework.

The region's carrier-slot mapping must be deterministic and equivalent to embedding into a standalone tile of the same dimensions using the same tile seed. This preserves crop-resistance semantics while avoiding a temporary pixel buffer.

Requirements:

- no `RgbaImage::new(tile_w, tile_h)` per embedded tile;
- no pixel-by-pixel crop followed by a second pixel-by-pixel blit for normal tile embedding;
- extraction should not allocate a complete temporary tile image for every candidate origin;
- partial edge tiles retain the existing capacity/skip policy;
- alpha remains untouched;
- legacy tiled extraction remains compatible if Plan 058 preserved a legacy scheme.

Required tests:

```text
tiled_direct_region_matches_reference_roundtrip
tiled_current_scheme_survives_aligned_crop
tiled_current_scheme_survives_misaligned_crop
tiled_legacy_fixture_still_extracts
tiled_partial_edge_behavior_unchanged
tiled_direct_region_does_not_touch_outside_region
```

A test-only reference crop implementation may remain to prove equivalence; it must not be used in production.

---

## 7. Phase 5 — remove low-value temporary bit allocations where simple

After the major changes, inspect:

```text
bytes_to_bits
bits_to_bytes
payload bit replication
DCT payload bit expansion
```

Prefer direct indexed bit access when it clearly reduces allocation and code remains readable, for example:

```text
bit(payload, i) = (payload[i / 8] >> (i % 8)) & 1
```

Do not replace clear code with complex iterator machinery solely to eliminate a small allocation outside a hot loop.

Required condition for such a change:

- it deletes at least one payload-sized allocation/copy in an embed/extract hot path;
- tests cover bit order exactly;
- legacy bit ordering remains unchanged.

This phase may be recorded `NO CHANGE NEEDED` if the extracted carrier implementation from Plan 059 already avoids the allocations cleanly.

---

## 8. Phase 6 — keep resource accounting truthful

If allocation patterns or capacity units change, update resource accounting only where the current report would otherwise become incorrect.

Do not invent a new performance telemetry subsystem.

At minimum verify:

- output allocation is still observed where expected;
- tile-origin bounds remain enforced;
- JPEG parser limits remain enforced before expensive work;
- no optimization bypasses input/dimension limits;
- capacity failure occurs before unnecessary mutation/encode work where possible.

---

## 9. Acceptance criteria

Plan 060 is complete only when:

1. Supported JPEG capacity/redundancy is selected deterministically before production re-encode attempts.
2. The normal successful supported-JPEG path decodes coefficients once and encodes the final JPEG once.
3. The normal successful path does not clone a full coefficient map for each lower redundancy candidate.
4. Focused round-trip tests prove one-pass carrier stability across the supported JPEG fixture matrix.
5. If any fallback remains, it is narrow, test-proven, and documented with the exact input class requiring it.
6. JPEG container preservation tests remain passing for unrelated APP/COM segments.
7. Progressive, restart-bearing, multi-scan, malformed, and otherwise unsupported JPEG behavior is not broadened accidentally.
8. Tiled LSB production embedding and extraction operate on image regions without allocating a complete temporary `RgbaImage` for each tile/origin.
9. Current tiled crop-resistance tests remain passing.
10. Legacy tiled compatibility from Plan 058 remains passing.
11. No optimization changes payload wire versions, rights semantics, HMAC/CRC rules, seed-discovery policy, or public API.
12. Any bit-vector allocation removal preserves exact bit order and is covered by focused tests.
13. The status ledger records before/after structural operation counts and, where available, representative benchmark observations.
14. No mandatory benchmark gate, CI expansion, dependency addition, release, or version change is introduced.
15. `./scripts/check.sh` passes before completion.

---

## 10. Stop conditions

Stop and record `PARTIAL` rather than forcing the optimization if:

- one-pass JPEG output is not stable for an input that the repository currently classifies as supported;
- removing retry logic would require broadening JPEG parsing/encoding scope;
- direct-region tiled traversal changes legacy/current carrier coordinates in a way that breaks compatibility;
- a proposed micro-optimization materially increases code complexity without measurable or structural benefit.

Correctness and maintainability outrank benchmark results.