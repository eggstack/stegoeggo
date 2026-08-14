# Plan 063 Status Ledger

Baseline SHA: `268f0a7` (main)

## Public API under `stegoeggo::stego` (from Plan 062)

```text
stego::StegoError          (error.rs)
stego::StegoResult         (error.rs)
stego::JpegUnsupportedReason (error.rs)
stego::CapacityReport      (mod.rs)
stego::EmbedReport         (mod.rs)

stego::lsb::LsbConfig      (lsb.rs)
stego::lsb::capacity()     (lsb.rs)
stego::lsb::embed()        (lsb.rs)
stego::lsb::extract()      (lsb.rs)
stego::lsb::DEFAULT_TILE_SIZE (lsb.rs)
stego::lsb::MIN_TILE_SIZE  (lsb.rs)
stego::lsb::tile_seed()    (lsb.rs)

stego::jpeg::JpegConfig    (jpeg.rs)
stego::jpeg::JpegSupport   (jpeg.rs)
stego::jpeg::probe_support() (jpeg.rs)
stego::jpeg::capacity()    (jpeg.rs)
stego::jpeg::embed()       (jpeg.rs)
stego::jpeg::extract()     (jpeg.rs)
stego::jpeg::embed_seed_hint() (jpeg.rs)
stego::jpeg::extract_seed_hint() (jpeg.rs)

stego::frame::FRAMED_MAGIC (frame.rs)
stego::frame::FRAME_VERSION (frame.rs)
stego::frame::MAX_FRAME_PAYLOAD (frame.rs)
stego::frame::FRAME_HEADER_SIZE (frame.rs)
stego::frame::FrameHeader  (frame.rs)
stego::frame::encode()     (frame.rs)
stego::frame::decode()     (frame.rs)
stego::frame::decode_prefix() (frame.rs)
```

## Source files implementing the carrier

```text
src/stego/mod.rs            (63 lines)
src/stego/error.rs          (135 lines)
src/stego/lsb.rs            (773 lines)
src/stego/jpeg.rs           (615 lines)
src/stego/frame.rs          (269 lines)
src/jpeg_transcoder/        ( substantial — header, entropy, stego_f5, mod.rs )
```

## Dependency classification (default/no-default-features)

### Direct normal dependencies (14)

| Dependency | Version | Carrier-required | Application-only |
|---|---|---|---|
| image | 0.25.6 | YES (pixel ops, JPEG decode) | — |
| jpeg-encoder | 0.7 | YES (JPEG encode) | — |
| crc32fast | 1.4 | YES (frame CRC32) | — |
| thiserror | 1.0 | YES (error derive) | — |
| serde | 1.0 | — | YES (type serialization) |
| serde_json | 1.0 | — | YES (JSON output) |
| quick-xml | 0.41 | — | YES (XMP parsing) |
| sha2 | 0.10 | — | YES (HMAC/MAC) |
| hmac | 0.12 | — | YES (HMAC/MAC) |
| hex | 0.4 | — | YES (hex encoding) |
| digest | 0.10 | — | YES (crypto traits) |
| subtle | 2.6 | — | YES (constant-time) |
| zeroize | 1.9 | — | YES (key zeroing) |
| getrandom | 0.2 | — | YES (CSPRNG seed) |

### Transitive count (cargo tree)

Current: ~40 transitive packages (default/no-default-features).

Carrier-only estimate: ~15 transitive packages (image, jpeg-encoder, crc32fast, thiserror + their transitive deps).

## Source boundary audit

### Carrier module dependencies on root crate

| Carrier module | Root dependency |
|---|---|
| lsb.rs | `crate::protected::constants::{SPLITMIX64_SEED, STEGO_OFFSET_SEED_1, STEGO_SPREAD_FACTOR}` |
| lsb.rs | `crate::types::{EmbedOutcome, EmbedPath}` |
| jpeg.rs | `crate::error::Error` (type alias + From impl) |
| jpeg.rs | `crate::jpeg_transcoder::{DctStegoF5, JpegHeader, JpegTranscoder}` |
| error.rs | `crate::Error` (via `From<StegoError> for crate::Error`) |
| frame.rs | None (self-contained) |
| mod.rs | None |

### Root crate dependencies on carrier-adjacent code

| Root module | Carrier-adjacent dependency |
|---|---|
| protected/steganography.rs | `jpeg_transcoder::{DctStegoF5, JpegHeader, JpegTranscoder}` (heavy) |
| protected/steganography.rs | Constructs `EmbedOutcome` directly for DCT paths |
| protected/steganography.rs | Uses `STEGO_OFFSET_SEED_1` for legacy extraction |
| lib.rs | Re-exports `jpeg_transcoder::is_progressive_jpeg` |
| lib.rs | Pattern-matches on `EmbedPath` for warnings |
| error.rs | `From<TranscoderError> for Error` |

### Application types NOT needed by carrier

ProtectionRequest, ResolvedProtectionPlan, ProtectionContext, RightsPolicy, DmiValue, LegalMetadata, StegoPayload, payload_v3 types, notice verification types — NONE of these are imported by `src/stego/`.

## Decision table

| Criterion | Observed current state | Benefit of split | Cost/risk of split | Disposition | Evidence |
|---|---|---|---|---|---|
| Carrier public API stability | Stable after Plan 062, no planned changes | None — API is already stable | Low — moving stable code is safe | SPLIT OK | Plan 062 complete |
| Dependency direction | N/A (single crate) | One-way `stegoeggo → carrier` achievable | Must ensure no cycle | SPLIT OK | jpeg_transcoder has no application deps |
| Application types in carrier | None | Preserved — no application types leak | None | SPLIT OK | Source audit above |
| Root re-export compatibility | `pub mod stego { pub use ... }` | Existing API preserved unchanged | Minor source churn for import paths | SPLIT OK | Mechanical re-export |
| Dependency removal value | 14 direct deps for generic consumer | Removes 10 application-only deps (serde, serde_json, quick-xml, sha2, hmac, hex, digest, subtle, zeroize, getrandom) | None for carrier consumer | SPLIT OK | Dependency tree measured |
| Release/CI complexity | Single crate, simple CI | One additional workspace member | No new CI/release workflow needed — existing check.sh covers workspace | SPLIT OK | check.sh already workspace-aware |
| Code duplication | N/A | Split eliminates shared code duplication | Root re-exports, no duplication | SPLIT OK | Mechanical re-export |
| Independent description | Generic stego carrier is coherent | Arbitrary-payload image steganography carriers | None | SPLIT OK | Clear module boundary |

## Decision: SPLIT

Rationale:
1. Removes 10 application-only direct dependencies from generic carrier consumers (14 → 4 direct deps).
2. Reduces transitive package count from ~40 to ~15.
3. Carrier boundary is clean — no application/rights types leak.
4. Root re-exports preserve the Plan 062 API unchanged.
5. `jpeg_transcoder` is a generic JPEG utility with no application dependencies — it moves cleanly with the carrier.
6. Existing `check.sh` is workspace-aware and covers the new crate.
7. No new CI/release/orchestration needed.
8. The carrier crate has a coherent independent description: arbitrary-payload image steganography carriers.

## Footprint measurement (Phase 5)

| Metric | Carrier (stegoeggo-stego) | Root (no-default-features) |
|---|---|---|
| Direct dependencies | 4 | 15 |
| Transitive packages | 32 | 77 |
| Direct deps removed from generic consumer | — | 11 (serde, serde_json, quick-xml, sha2, hmac, hex, digest, subtle, zeroize, getrandom, ed25519-dalek) |

## Implementation plan

1. Create `stegoeggo-stego/` workspace member with `Cargo.toml` + `src/lib.rs`
2. Move `src/stego/` → `stegoeggo-stego/src/` (lsb.rs, jpeg.rs, frame.rs, error.rs, mod.rs)
3. Move `src/jpeg_transcoder/` → `stegoeggo-stego/src/jpeg_transcoder/`
4. Move carrier-required constants from `src/protected/constants.rs` to carrier
5. Move `EmbedOutcome`/`EmbedPath` types to carrier
6. Create `From<StegoError> for crate::Error` in root (reversed direction)
7. Create `From<TranscoderError> for crate::Error` in root
8. Update all `crate::jpeg_transcoder` imports in root to use carrier
9. Re-export `stegoeggo_stego` API under `stegoeggo::stego`
10. Update workspace Cargo.toml
11. Run check.sh, fix issues
12. Update docs
