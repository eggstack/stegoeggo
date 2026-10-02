# Stego Library Evolution Milestone 005 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/stego-library-evolution/005-lossless-webp-byte-carrier.md`
Source subsystem roadmap: `plans/subsystems/stego-library-evolution-roadmap.md#7-milestones`
Repository baseline reviewed: `1175f63` (M004 closure; plan baseline `d5425d3` plus M001–M004 work)
Implementation commit: `a70aa75` — stego: add opt-in still-lossless WebP byte carrier facade
Closing commit: this record plus roadmap/registry reconciliation (see git log for `plans: close stego-library-evolution M005 lossless WebP byte carrier`).

## 1. Executive finding

A direct carrier-crate consumer can now embed/extract arbitrary bytes in
a still VP8L file without handling pixels: the opt-in `webp` feature
adds a `stegoeggo_stego::webp` facade (probe, capacity, raw/framed/
tiled embed/extract) that decodes to RGBA, reuses the LSB carrier core
with no forked permutation logic, and re-encodes VP8L losslessly. Only
still lossless input is accepted — lossy VP8 and animation (flag or
ANMF frames) are rejected with structured `UnsupportedWebP`
classification, never transcoded. Every operation takes
`CarrierLimits`, checks input bytes, canvas/bitstream dimensions,
framed totals, and tiled origins before any large allocation, and is
all-or-error with no partial output. Unrelated container metadata is
dropped without claim, and root rights byte paths are untouched. M005
closes; M001–M005 all have closure evidence. M006 stays blocked on
ADR-0007 acceptance.

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Opt-in carrier feature forwarding `image/webp`, off by default unless the codec is already unavoidable (§6) | `webp = ["image/webp"]`, off by default; codec already in the workspace lock via the parent (`image` with `png`/`jpeg`/`webp`); default tree unchanged, `--no-default-features` green (§4) |
| Small `stegoeggo_stego::webp` facade reusing `lsb`/pixel core, no forked permutation (§6) | 10 public functions delegating to `lsb::` on decoded RGBA; zero carrier-algorithm code duplicated (§3) |
| RIFF/VP8X probe distinguishing still VP8L from VP8/animation using M003 limits (§6) | Bounded chunk walk: input-bytes gate, exact RIFF-size match, per-chunk checked arithmetic, canvas + bitstream dimension checks; VP8X parsed as 10-byte feature header with image/metadata chunks as top-level siblings (spec layout, verified against the `image-webp` 0.2.4 parser, which scans post-VP8X siblings) (§3, §4) |
| Decode to RGBA, same LSB mapping, VP8L-lossless encode (§6) | `decode_still` → `to_rgba8` → `lsb::` → `WebPEncoder::new_lossless`; RGB/RGBA/alpha round trips green (§4) |
| Explicit metadata contract; minimum = new lossless carrier, no preservation promise (§6) | Module docs, README, guide, and STABILITY state the drop contract; XMP-drop pinned by test (§4) |
| Raw/framed/tiled convenience only where semantics map to LSB (§6) | Framed mirrors `lsb::extract_framed` capacity gating plus `check_frame_bytes`; tiled mirrors `lsb::` with `check_tiled_origins` first; no new semantics invented (§3) |
| Direct consumer embeds/extracts without handling pixels; lossy/animation explicit; no accidental default surface (§13) | 16-test direct-consumer fixture; negative-format matrix; default-feature build proves no surface change (§4) |

## 3. Production implementation evidence

Implementation `a70aa75` (13 files):

- `stegoeggo-stego/Cargo.toml`: `webp = ["image/webp"]` (off by default).
- `stegoeggo-stego/src/webp.rs` (new, ~700 lines with 16 unit tests):
  `WebpSupport::StillLossless { width, height, has_alpha }`,
  `probe_support`, `capacity`, `embed`, `extract`, `embed_framed`,
  `extract_framed`, `embed_tiled`, `extract_tiled`,
  `embed_tiled_framed`, `extract_tiled_framed`, all taking
  `limits: &CarrierLimits`. Internal bounded RIFF walk (`chunk_at` /
  `chunk_next` with checked arithmetic, monotonic advance, tolerant
  terminal pad), VP8L bit-header dimension parse, VP8X feature-header
  parse with sibling scan (VP8 → `LossyVp8`, animation flag or ANMF →
  `Animated`, second VP8L → malformed, none → `MissingLosslessImageData`),
  pre-decode dimension gates, decoder animation re-check, post-decode
  dimension gate, `total_bytes`-sized single allocation, VP8L-output
  shape assertion on encode.
- `stegoeggo-stego/src/error.rs`: `StegoError::UnsupportedWebP` +
  `WebpUnsupportedReason::{LossyVp8, Animated, MissingLosslessImageData}`
  (`#[non_exhaustive]` enum, additive) with `Display`.
- `stegoeggo-stego/src/lib.rs`: `#[cfg(feature = "webp")] pub mod webp`,
  facade-docs bullet, root re-export of the reason type.
- `Cargo.toml` (root): opt-in `webp = ["stegoeggo-stego/webp"]`;
  `src/lib.rs`: gated `pub use stegoeggo_stego::webp` plus unconditional
  reason-type re-export. No pipeline, protector, or CLI change: root
  rights byte paths cannot reach the facade implicitly.
- `stegoeggo-stego/tests/webp_carrier.rs` (new, `#![cfg(feature =
  "webp")]`): 16 direct-consumer tests (§4).
- Docs: module rustdoc with compiling example, carrier README (carrier
  table row, WebP section, error list), `docs/carrier-crate.md`
  (facade contract), `STABILITY.md` (additive-stable listing + module
  table), `architecture/carrier-surface.md` (facade section + report
  output note).

No mapping, redundancy, seed-derivation, capacity-unit, report-shape,
or default-feature change. `Cargo.lock` unchanged (codec already
locked).

## 4. Verification executed (exact commands + results)

- `cargo test -p stegoeggo-stego --features webp` → 200 lib + 13 direct-consumer + 1 peak + 16 webp_carrier + 36 doctests passed, 0 failed: RGB/RGBA raw round trips, redundancy round trip via report value, RGB sourc
...[truncated 2085 chars]