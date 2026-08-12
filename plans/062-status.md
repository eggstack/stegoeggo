# Plan 062: Status

Baseline SHA: `5ab0bae` (Plan 061 complete)

## Semver Review

This plan is **additive** to the existing public API surface. No existing public symbols are removed or renamed.

### Existing public stego symbols (pre-062)

| Symbol | Location | Notes |
|--------|----------|-------|
| `SteganographyProtector` | `crate::protected::steganography` | Application-specific orchestrator, unchanged |
| `StegoPayload` | `crate::protected::steganography` | Application-specific payload, unchanged |
| `tile_seed` | `crate::stego::lsb` (re-exported via `protected::steganography`) | Already public |
| `DEFAULT_TILE_SIZE` | `crate::stego::lsb` (re-exported via `protected::steganography`) | Already public |
| `is_progressive_jpeg` | `crate::jpeg_transcoder` (re-exported from `lib.rs`) | Unchanged |

### New public symbols (062)

All new. Additive. No existing callers affected.

| Symbol | Module | Purpose |
|--------|--------|---------|
| `stego::lsb::LsbConfig` | `crate::stego::lsb` | Configuration for LSB carrier operations |
| `stego::lsb::capacity` | `crate::stego::lsb` | Query available capacity for an RGBA image |
| `stego::lsb::embed` | `crate::stego::lsb` | Embed arbitrary bytes into RGBA image |
| `stego::lsb::extract` | `crate::stego::lsb` | Extract arbitrary bytes from RGBA image |
| `stego::jpeg::JpegConfig` | `crate::stego::jpeg` | Configuration for JPEG DCT carrier operations |
| `stego::jpeg::capacity` | `crate::stego::jpeg` | Query available capacity for JPEG bytes |
| `stego::jpeg::embed` | `crate::stego::jpeg` | Embed arbitrary bytes into JPEG |
| `stego::jpeg::extract` | `crate::stego::jpeg` | Extract arbitrary bytes from JPEG |
| `stego::jpeg::JpegSupport` | `crate::stego::jpeg` | JPEG structure support classification |
| `stego::jpeg::embed_seed_hint` | `crate::stego::jpeg` | Embed seed in Q-tables |
| `stego::jpeg::extract_seed_hint` | `crate::stego::jpeg` | Extract seed from Q-tables |
| `stego::frame::FrameHeader` | `crate::stego::frame` | Generic self-describing payload frame |
| `stego::frame::encode` | `crate::stego::frame` | Encode payload into framed bytes |
| `stego::frame::decode` | `crate::stego::frame` | Decode framed bytes, verify CRC |
| `stego::frame::FRAMED_MAGIC` | `crate::stego::frame` | 2-byte magic identifier |
| `stego::frame::FRAME_VERSION` | `crate::stego::frame` | Current frame version |
| `stego::StegoError` | `crate::stego` | Structured error for generic carrier ops |
| `stego::CapacityReport` | `crate::stego` | Capacity query result |
| `stego::EmbedReport` | `crate::stego` | Embedding outcome report |

## Frame Wire Format

```text
Offset  Size  Field
0       2     Magic [0x53, 0x47] ("SG" for StegoEggo Generic)
2       1     Version (=1)
3       4     Payload length (u32 LE, max 16 MiB)
7       4     CRC32 of payload bytes
11..    N     Payload bytes
```

Total overhead: 11 bytes. Max payload: 16,777,216 bytes (16 MiB).
CRC32 covers payload bytes only (not header).
Trailing bytes after complete frame are rejected as malformed.

## Decisions

1. **Legacy scheme not exposed** — Only the corrected V2 carrier scheme is exposed publicly. Legacy V1 probing is application-private.
2. **JPEG seed hint is public** — `embed_seed_hint`/`extract_seed_hint` are exposed with explicit naming and fragility warnings in rustdoc.
3. **`intensity` not exposed** — No public config includes `intensity`. Redundancy is the explicit capacity/cost control.
4. **Error model** — `StegoError` is a standalone enum with `From<StegoError> for Error` conversion. No new variants added to the root `Error` enum.
5. **Frame is optional** — Raw APIs don't require framing. Frame is a composition convenience.
6. **No tiled public API** — Tiled embedding is retained internally for crop resistance but not exposed as a public generic API in this plan. The internal tiled path continues to work.
7. **`actual_redundancy` in EmbedReport** — JPEG embed auto-downgrades redundancy when capacity insufficient. EmbedReport includes `actual_redundancy` so callers can pass it to extract for correct decoding.

## Completion Status

- [x] Phase 0: API inventory, semver review, status ledger
- [x] Phase 1: Public raw LSB API (LsbConfig, capacity, embed, extract)
- [x] Phase 2: Public encoded-JPEG API (JpegConfig, probe_support, capacity, embed, extract)
- [x] Phase 3: Capacity and embed result types (CapacityReport, EmbedReport)
- [x] Phase 4: Explicit JPEG seed-hint API (embed_seed_hint, extract_seed_hint)
- [x] Phase 5: Minimal generic frame (FrameHeader, encode, decode, decode_prefix)
- [x] Phase 6: Framed convenience APIs (LSB and JPEG framed roundtrip tests)
- [x] Phase 7: Error model (StegoError, JpegUnsupportedReason)
- [x] Phase 8: Documentation (rustdoc, AGENTS.md, architecture/protected-steganography.md)
- [x] Phase 9: Required tests (29 public API tests in tests/public_stego_api.rs)
- [x] ./scripts/check.sh passes
- [x] plans/062-status.md records final public API inventory, frame wire spec, and semver review
- [x] frame::decode rejects trailing bytes after complete frame
- [x] README "Generic carrier API" section added
- [x] examples/generic_stego.rs created
