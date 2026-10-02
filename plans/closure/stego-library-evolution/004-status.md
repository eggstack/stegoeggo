# Stego Library Evolution Milestone 004 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/stego-library-evolution/004-transactional-in-place-allocation-hardening.md`
Source subsystem roadmap: `plans/subsystems/stego-library-evolution-roadmap.md#7-milestones`
Repository baseline reviewed: `58a3743` (M003 closure; plan baseline `d5425d3` plus M001–M003 work)
Implementation commit: `6616db2` — stego: make tiled in-place embed transactional without rollback clone
Closing commit: this record plus roadmap/registry reconciliation (see git log for `plans: close stego-library-evolution M004 transactional in-place allocation hardening`).

## 1. Executive finding

The full-image rollback clone in `lsb::embed_tiled_in_place` is gone,
replaced by the plan's first-attempt strategy that the failure-point
analysis proved viable: a complete checked preflight (viable tiles,
exact capacity, permutation-domain check) resolves every mutation
failure condition before the first pixel write, so the commit phase
cannot fail on a live carrier and no rollback state is needed. Peak
auxiliary allocation on a 1 MiB image fell from 1,053,952 bytes to
5,376 bytes (196x) with no runtime regression on the affected path and
byte-identical success/failure semantics. No journal was introduced, so
no test-only failure hook was needed. M004 closes; M005 was already
ready and is unaffected; M006 stays blocked on ADR-0007 acceptance.

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Complete checked preflight before the first write; remove the full snapshot if provable (§6) | `embed_tiled_carrier` preflight now checks viable tiles, exact `bit_len * 5` capacity (saturation replaced by a checked gate returning the same MAX-shaped report), and per-tile `checked_next_power_of_two`; `lsb.rs` snapshot/restore deleted (§3) |
| Journal, if used, bounded by checked arithmetic and proven smaller than the image (§6) | No journal introduced: the preflight/commit split proved sufficient, so no journal bound was needed |
| One logical carrier mapping shared between `RgbaImage` and `PixelViewMut` (§6) | Unchanged: all tiled entry points (`lsb::embed_tiled`, `lsb::embed_tiled_in_place`, `PixelViewMut::embed_tiled`) share `embed_tiled_carrier`; no mapping code touched |
| Full-image rollback clone gone or justified as lower-risk/lower-memory (§13) | Gone: `let snapshot = img.clone()` plus conditional restore removed; peak-aux regression test pins the new bound (§4) |
| Atomic failure regression-tested (§13) | Pre-existing insufficient-capacity/exact-boundary atomicity tests plus new exact-fit and peak tests, all green (§4) |
| Successful output byte-compatible (§13) | In-place vs cloning parity asserted byte-for-byte (raw and exact-fit), framed/raw round trips green (§4) |
| Peak auxiliary memory measured, not inferred (§13) | Counting-allocator integration test: 1,053,952 → 5,376 bytes on a 1,048,576-byte image (§4) |

## 3. Production implementation evidence

Implementation `6616db2` (6 files):

- `stegoeggo-stego/src/lsb_internal.rs` (`embed_tiled_carrier`): exact
  `tile_required = bit_len * 5` computed once with a checked gate (the
  per-tile saturating `lsb_required_capacity_v2(bit_len, 1)` call is
  replaced; identical values on all live inputs), and the tile-viability
  condition gains `tile_available.checked_next_power_of_two().is_some()`.
  The mutation loop is otherwise untouched; its defensive
  `embedded: false` arms are now unreachable on live carriers (proof in
  §5) but retained as graceful degradation.
- `stegoeggo-stego/src/lsb.rs` (`embed_tiled_in_place`): snapshot clone
  and conditional restore removed; rustdoc now states the
  preflight/commit guarantee instead of rollback.
- `stegoeggo-stego/tests/tiled_in_place_peak.rs` (new): global counting
  allocator confined to this test binary, warm-up call before counter
  reset, 512x512 textured image with 36-byte payload and tile 64,
  asserts peak aux < image_bytes / 4 and extraction round trip.
- `stegoeggo-stego/src/lsb.rs` tests: new
  `tiled_in_place_exact_fit_embeds_and_roundtrips` (307-byte payload =
  12,280 required vs 12,288 available slots on a single 64x64 tile).
- Docs: carrier README in-place section (tiled preflight guarantee),
  `architecture/carrier-lsb.md` (transactional semantics + measured
  figures), `architecture/tooling.md` (bench method + before/after).

No mapping, redundancy, seed-derivation, capacity-unit, API-signature,
or report-shape change. The cloning `embed_lsb_tiled` output clone is
the return value, not rollback state, and is unchanged.

## 4. Verification executed (exact commands + results)

- `cargo test -p stegoeggo-stego --all-features` → 196 lib + 13 direct-consumer + 1 peak + 35 doctests passed, 0 failed (includes new exact-fit test and peak regression test).
- New peak test before fix: `peak aux 1053952 exceeds a quarter of image bytes 1048576` (fails as designed); after fix: `tiled in-place peak aux bytes: 5376 (image bytes: 1048576)` (passes, ~194x margin under the bound).
- `cargo bench -p stegoeggo --bench bench -- --noplot tiled_lsb_request` → `png_tiled/256` 1.863 ms (baseline 1.866 ms), `png_tiled/1024` 31.21 ms (baseline 31.50 ms): no runtime regression on the affected path. One `tiled`-filter run showed an outlying `png_tiled/1024` median with a 3x confidence-interval spread; an immediate re-run returned to the baseline band (recorded as machine noise in `tooling.md`).
- `cargo bench -p stegoeggo --bench bench -- --noplot tiled` and `-- --noplot lsb_clone_vs_in_place` → JPEG tiled and non-tiled LSB paths (untouched by this diff) within run-to-run noise.
- `./scripts/check.sh` → exit 0 (fmt, clippy `-D warnings`, no-default-features check, all suites green, docs contract valid).

## 5. Invariant review

- Any returned non-success leaves caller bytes unchanged: every
  `embedded: false` site in `embed_tiled_carrier` precedes the first
  write (scan-phase capacity/dimension/arithmetic gates) except the
  defensive mutation-loop arms, which §6 analysis proves unreachable:
  on a planned tile `logical < tile_required <= tile_available` with
  exact arithmetic; `stego_permutation_v2` is total there (slot count
  nonzero, above 1, power-of-two-checked); slot-to-pixel resolution
  satisfies `slot / 3 < sub_w * sub_h` exactly; the `u32` coordinate
  conversion is in-range in both the truncating and non-truncating
  cases; window offsets cannot overflow (`x0 + lx < x1 <= width`).
  Resolution reads no pixel data, so dry-run and commit agree by
  determinism; all three `PixelCarrierMut` implementors (the traits are
  `pub(crate)`, no external impls possible) bounds-check coordinates
  only.
- Successful bytes compatible: in-place/cloning parity byte-for-byte,
  known-answer and robustness suites green via `check.sh`.
- Alpha/padding untouched: no mapping change; view alpha/padding tests green.
- No unsafe code: `#![forbid(unsafe_code)]` retained in library code
  (the counting allocator lives in an integration-test binary, outside
  the forbidding crates).
- Overflow/capacity checks before unchecked indexing/allocation:
  strengthened (exact capacity gate, permutation-domain gate).

## 6. Failure and recovery review

The operation remains synchronous and caller-owned with no
cancellation. All error and `embedded == false` paths return before
any mutation; the removed restore branch is subsumed by the preflight
proof. Panic is still not a rollback mechanism: no `expect`/`unwrap`
was added to the mutation path, and the defensive arms degrade to
`embedded: false`.

## 7. Migration and compatibility review

No API migration: signatures, report shapes, error variants, capacity
values, and embedded bytes are unchanged on all live inputs. The two
added preflight gates trigger only on inputs no live carrier can
present (payload near `usize::MAX` bytes, carrier above exabyte scale),
where they return the same report shapes the old saturation paths
produced. The parent raster path (`embed_raster_with_seed_fallback`)
inherits the guarantee with no change.

## 8. Security review

No secret handling changed. The removed clone also removes a transient
full-image copy from memory. No new allocation scales with untrusted
input beyond the pre-existing per-tile scan ledger (`O(tile count)`,
no per-slot storage).

## 9. Documentation and operations

Rustdoc (`embed_tiled_in_place`), carrier README, `carrier-lsb.md`,
and `tooling.md` document the guarantee, the measured figures, and the
bench method. No operator action required.

## 10. Unresolved findings (critical/high/medium/low)

None. The §16 no-op exit was not needed: measurement confirms the
snapshot was pure overhead (99.5% of peak aux) with a provably
infallible commit phase.

## 11. Roadmap disposition

Stego-library-evolution M004 closed. M005 (lossless WebP byte facade,
ready) is unaffected and proceeds independently. M006 stays blocked:
ADR-0007 remains proposed. M007 remains proposed future research.

## 12. Registry updates

`registry.md`: stego current milestone advanced past M004 (M005 ready
remains); M004 recorded under recently closed work. Roadmap status
table marks M004 closed with this closure record.
