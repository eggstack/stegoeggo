# Stego Library Evolution Milestone 003 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/stego-library-evolution/003-carrier-resource-and-prepared-hardening.md`
Source subsystem roadmap: `plans/subsystems/stego-library-evolution-roadmap.md#7-milestones`
Repository baseline reviewed: `6d87c57` (M003 implementation; plan baseline `d5425d3` plus M001 `ed1dd6d`/`9c304bc`, M002 `beb4145`/`3424a23`, and this milestone's carrier-owned limits plus prepared-state work)
Implementation commits: `6d87c57` — stego: harden carrier with owned limits and prepared tiled parity
Closing commit: this record plus roadmap/registry reconciliation (see git log for `plans: close stego-library-evolution M003 carrier resource and prepared hardening`).

## 1. Executive finding

Generic carrier consumers have an explicit bounded-untrusted-input
contract, and repeated JPEG operations reuse the single decoded
coefficient state without exposing codec internals: carrier-owned
`CarrierLimits` (private fields, getters, builder) bounds input bytes,
JPEG segments, dimensions/pixels, framed totals, and tiled-search
extent; bounded `*_with_limits` one-shot variants and
`PreparedJpeg::new_with_limits` share the single decode path with the
defaults so results agree exactly; `PreparedJpeg` gains tiled exact
embed (`embed_tiled` / `embed_tiled_framed`) matching the one-shot
contract with no best-effort degradation; limit failures map to
`StegoError::ResourceLimitExceeded` with no secret/input dump. Default
one-shot behavior is preserved for ordinary inputs. M003 closes and
unblocks M004 and M005; M006 stays blocked on ADR-0007 acceptance.

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Carrier-owned limits type, private fields/builders, covering input bytes / segments / dimensions-pixels / frame bytes / tiled bound (§6.1) | `stegoeggo-stego/src/limits.rs`: `CarrierLimits` + `CarrierLimitsBuilder`, defaults 100 MiB / 256×65535 / 16384×16384 / 16 MiB+11 / 4096; unit tests pin defaults, builder round-trip, and check mapping (§3) |
| `PreparedJpeg::new_with_limits` plus bounded one-shot variants without duplicating decode code (§6.2) | `new_with_limits` + 13 `*_with_limits` JPEG variants (`capacity`, `embed_best_effort`, `embed_strict`, `embed_framed_best_effort`, `embed_framed_strict`, `extract`, `extract_framed`, `embed_tiled`, `embed_tiled_framed`, `extract_tiled`, `extract_tiled_framed`, `probe_support`, `inspect_with_limits`); one-shot defaults delegate to the same `decode_supported_carrier_with_limits` core (§3) |
| Limits threaded through private header/coefficient paths before large work (§6.3) | Bounded decode checks input bytes, parses headers with segment limits, checks dimensions/pixels, then decodes coefficients via the new `decode_coefficients_with_probe_and_limits`; framed totals checked before full extraction; tiled origins checked before search (§3) |
| `jpeg::inspect` compatible, may delegate through the new object (§6.4) | Signature unchanged; delegates through `CarrierLimits` builder values into `inspect_with_limits`; parent `resource_limits.rs` segment tests still assert the same limit messages (§4) |
| Prepared operations only reusing decoded state with exact one-shot match; tiled exact embed candidate; no best-effort degradation on `PreparedJpeg` (§6.5) | `embed_tiled_from_decoded` core shared by one-shot and prepared; `PreparedJpeg::embed_tiled` / `embed_tiled_framed` match bytes; no `embed_best_effort` on the handle (§3, §4) |
| Limit failures to `ResourceLimitExceeded`, no secret/input dump (§6.6) | All `check_*` mappers emit counts/limits only; seed/config values never formatted; adversarial tests assert the variant (§4) |
| Network-facing consumer can bound parsing through public API (§13) | `CarrierLimits` + `*_with_limits` + `new_with_limits` demonstrated in README, `docs/carrier-crate.md`, and `examples/generic_stego.rs` (§9) |
| Prepared reuse opaque and parity-tested; limit failures deterministic (§13) | Default-limit parity tests across all bounded variants; prepared tiled byte parity; reuse-after-failure tests; `max_origins`/`frame`/`input`/`segment`/`dims` limit tests deterministic (§4) |

## 3. Production implementation evidence

Implementation `6d87c57` (17 files):

- New `stegoeggo-stego/src/limits.rs`: `CarrierLimits` (private fields, 8 getters, `builder()`, `Default`, `parse_limits`/`check_input_bytes`/`check_dimensions`/`check_frame_bytes`/`check_tiled_origins` as `pub(crate)`), `CarrierLimitsBuilder` (8 `#[must_use]` setters + `build`), 3 unit tests.
- `stegoeggo-stego/src/lib.rs`: `pub mod limits` + `CarrierLimits`/`CarrierLimitsBuilder` re-exports + stable-API docs.
- `src/lib.rs`: `stego::limits` + `CarrierLimits`/`CarrierLimitsBuilder` facade re-exports.
- `stegoeggo-stego/src/jpeg_transcoder/mod.rs`: `decode_coefficients_with_probe_and_limits`; the existing probe delegates with default `ParseLimits` (no behavior change).
- `stegoeggo-stego/src/jpeg.rs`: bounded decode core; `inspect` delegates through `CarrierLimits`; 13 `*_with_limits` variants; `embed_tiled_from_decoded` core shared by one-shot/bounded/prepared; tiled framed limit-aware candidate skip; `probe_support_with_limits`; `map_decode_error` dead helper removed and now-unused `pub(crate)` non-limits extract helpers removed.
- `stegoeggo-stego/src/prepared.rs`: `limits` stored on the handle, `new` delegates with defaults, `new_with_limits` + `limits()` getter, framed/tiled extracts and framed embeds enforce handle limits, new `embed_tiled` / `embed_tiled_framed` with exact one-shot parity.
- `stegoeggo-stego/src/application_support.rs`: decode-count test re-anchored after fixture setup (the shared counted helper now also counts embed-path decodes; search-context single-decode assertion unchanged in intent).
- Tests: `direct_consumer.rs` gains default-parity, adversarial-bounds, and prepared-tiled suites; `tests/public_stego_api.rs` gains facade parity/bounds; `tests/robustness.rs` gains a `carrier_limits` adversarial regression module (input/segment/dims/frame/origins/overflow/truncation, all panic-guarded).
- Example: `examples/generic_stego.rs` demonstrates `capacity_with_limits`, `new_with_limits`, and prepared `embed_tiled`.
- Docs: carrier README (bounded section + prepared + API summary), `docs/carrier-crate.md` (prepared + bounded sections), `STABILITY.md` (additive bounded/prepared surface), `architecture/carrier-jpeg.md` + `carrier-surface.md` (bounded rows + prepared tiled), `architecture/resource-limits.md` (carrier/root disposition split).

No existing defaults changed: default `CarrierLimits` equal current hard bounds (segments 256×65535 identical to `ParseLimits::default`; frame 16 MiB+11 identical to frame codec bound; tiled 4096 identical to `MAX_TILED_ORIGINS`); ordinary inputs decode identically. `inspect` limit-exceed disposition intentionally moves from `MalformedInput` to `ResourceLimitExceeded` per the plan's mapping rule; messages preserved so the parent segment tests still assert the same text.

## 4. Verification executed (exact commands + results)

- `cargo test -p stegoeggo-stego --all-features` → 195 lib + 13 direct-consumer + 35 doctests passed, 0 failed (includes limits unit tests, bounded parity/adversarial/prepared-tiled suites).
- `cargo test --workspace --all-features --test public_stego_api` → 56 passed, 0 failed (includes facade bounded parity/bounds).
- `cargo test --workspace --all-features --test known_answer_vectors` → 10 passed, 0 failed (no carrier byte change).
- `cargo test --workspace --all-features --test robustness` → 71 passed, 0 failed (includes the 2 new `carrier_limits` adversarial regression tests).
- `cargo semver-checks check-release -p stegoeggo-stego` (cargo-semver-checks 0.50.0) → 196 checks pass, 58 skip, summary no semver update required (additive-only evolution; recorded although not required by this plan's §15).
- Fuzz (nightly-2026-09-07, `CARGO_PROFILE_RELEASE_LTO=false`): `cargo fuzz run jpeg_parser -- -max_total_time=30` → 1,531,200 runs, no crash; `cargo fuzz run tiled_round_trip -- -max_total_time=30` → 1,323,044 runs, no crash.
- `./scripts/check.sh` → exit 0 (fmt, clippy `-D warnings`, no-default-features check, 2054 passed / 0 failed / 25 ignored across 45 suites, docs contract valid: 5 release targets).

## 5. Invariant review

- Limits checked before allocation/work from untrusted lengths: input bytes before header parse; segments during header parse; dimensions/pixels before coefficient decode; framed totals before full extraction; tiled origins before search.
- Default one-shot APIs retain existing behavior: defaults equal current bounds; parity tests assert identical bytes/reports across every bounded variant under default limits; known-answer suite green.
- Prepared/one-shot results identical: capacity/extract/framed/tiled/strict/tiled-embed parity asserted in carrier, facade, and prepared unit suites.
- Unsupported JPEG classification stable: `probe_support` logic untouched; bounded probe adds only limit checks.
- `#![forbid(unsafe_code)]` retained; rustfmt 4-space/100-col clean; clippy `-D warnings` clean.

## 6. Failure and recovery review

Limit failure returns before output mutation/encoding where possible (decode-stage checks precede coefficient mutation and encode; framed checks precede full extraction). Prepared handles remain reusable after failed queries/embeds (capacity unchanged after oversized tiled attempt; extract parity after failure). No global mutable budget. Best-effort application degradation was not added to `PreparedJpeg` (only exact `embed_tiled`/`embed_tiled_framed`).

## 7. Migration and compatibility review

All existing functions remain with unchanged signatures; new bounded APIs are additive. `inspect` keeps its signature; only the limit-exceed error variant changes as required by the plan. The carrier never depends on the root `ResourceLimits`; the root may later translate its policy into carrier limits privately. No serialization, seed, redundancy, or capacity-unit change.

## 8. Security review

No secret handling changed (seed remains `u64`); limit messages carry counts/limits only, never seed bytes, keys, or input content. Bounded parsing closes the adversarial amplification points inventoried in WP1 (input bytes → segment walk → dimensions → coefficient decode → framed allocation → tiled search). Fuzz evidence shows no parser crash across ~2.85M combined runs on the two most relevant targets.

## 9. Documentation and operations

Carrier README/rustdoc, `docs/carrier-crate.md`, `STABILITY.md`, `examples/generic_stego.rs`, and `architecture/carrier-jpeg.md` + `carrier-surface.md` + `resource-limits.md` document the bounded contract, prepared tiled parity, and the carrier/root limits disposition. No operator action required. M005 implementers use `CarrierLimits` for bounded RIFF chunk traversal.

## 10. Unresolved findings (critical/high/medium/low)

None.

## 11. Roadmap disposition

Stego-library-evolution M003 closed. M004 (transactional in-place
allocation hardening, hard-blocked on M003) is unblocked and promotes to
ready: its preflight/journal work needs no further interface from this
workstream. M005 (lossless WebP byte facade, interface-blocked on the
M003 limits contract) is unblocked and promotes to ready: the
`CarrierLimits` vocabulary (input bytes, dimensions/pixels, frame bytes)
is available for its bounded RIFF probe. M006 stays blocked: M002/M003
are now closed but ADR-0007 remains proposed, and no implementation may
freeze the keyed-placement contract before acceptance. M007 remains
proposed future research.

## 12. Registry updates

`registry.md`: M003 row removed from dependency-ready plans; stego M004
and M005 promoted blocked→ready with plans linked; subsystem current
milestone advanced to M004/M005 ready; M004/M005 rows removed from
blocked work (M006 ADR-0007 row retained); M003 recorded under recently
closed work. Roadmap status table marks M003 closed with this closure
record and M004/M005 ready.
