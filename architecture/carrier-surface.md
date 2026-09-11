# Carrier Surface (Shared Reports, Frame, Types)

**Sources:** `stegoeggo-stego/src/lib.rs` (155) · `types.rs` (518) · `frame.rs` (301) · `prepared.rs` (bridge, see `carrier-jpeg.md`) · `error.rs` (121) · `constants.rs` (19) · `application_support.rs` (664, feature-gated hidden).

The stable generic surface re-exported as `stegoeggo::stego`. For standalone generic use depend on `stegoeggo-stego` directly — it is the documented public home. `#![forbid(unsafe_code)]`.

## Reports (`lib.rs`)

- `CapacityReport { required, available }` — carrier units per family (LSB: RGB slots; JPEG: eligible AC coefficients `|coef| >= 2`). `is_sufficient()` is a direct comparison.
- `EmbedReport<T> { embedded, output, payload_bytes, required_capacity, available_capacity, actual_redundancy }` — `output` is `RgbaImage` for LSB, `Vec<u8>` for JPEG. `payload_bytes` includes frame overhead for framed ops. Public fields frozen through 0.x; `into_output()` / `into_parts()` / `capacity()` helpers. Pass `actual_redundancy` to JPEG `extract`.
- `InPlaceEmbedReport` / `EmbedOutcome<T::{Embedded, SkippedCapacity, UnsupportedProgressive}>` / `EmbedStatus` / `EmbedOutcomeSummary { status, path, payload_bytes, required, available }` / `EmbedPath::{Lsb, LsbTiled, DctF5, DctF5Tiled, QTableSeedOnly}` follow the operation actually executed.

## Validated primitives (`types.rs`)

- `Redundancy(u8)` — `1..=10`, `new` / `from_usize` / `get` / `MIN` / `MAX`, identical debug/release semantics. Use `LsbConfig::from_redundancy` / `JpegConfig::from_redundancy` + `with_redundancy_value` for runtime values.
- `TileConfig { seed, tile_size }` — `try_new`; `MAX_TILED_ORIGINS: u32 = 4096` hard bound, `max_origins = 0` is `InvalidConfig`.

## Frame (`frame.rs`)

Self-describing TLV wrapper: `FRAMED_MAGIC [0x53,0x45]`, `FRAME_VERSION`, `FRAME_HEADER_SIZE = 11`, `MAX_FRAME_PAYLOAD = 16 MiB`. `encode(payload)` / `decode(data)` / `decode_prefix(data)`. CRC32 is corruption detection, not authentication. Framed extraction keeps seed/config explicit and validates capacity before full extraction.

## Errors (`error.rs`)

`StegoError::{InvalidConfig, InsufficientCapacity, MalformedInput, UnsupportedJpeg, FrameNotFound, MalformedFrame, FrameChecksumMismatch, ResourceLimitExceeded, EmptyCarrier}` + `JpegUnsupportedReason` (10 variants) + `StegoResult<T>`. The `stego` surface uses `StegoError`, not root `Error` (convert via `From`).

## Constants (`constants.rs`, `frame.rs`, `types.rs`)

`STEGO_OFFSET_SEED_1`, `STEGO_SPREAD_FACTOR = 5`, `SPLITMIX64_SEED`, `MIN/MAX_REDUNDANCY = 1/10`. V3 wire constants (`V3_MAGIC`, `V3_CORE_SIZE = 32`, …) live in the root crate's `payload_v3/types.rs`, not here.

## Hidden bridge (`application_support.rs`)

Feature `application-support`, `#[doc(hidden)]`, parent-crate only — never ordinary embed/extract. Holds legacy V1 extraction (`legacy_lsb_required_slots`, `legacy_lsb_extract{,_range}`), historical seed fallback (`seed_fallback_embed/extract`), tile-seed derivation, and single-decode search reuse (`TiledJpegSearch`, `TiledJpegCandidateKey`, `JpegSearchContext`). Private `jpeg_transcoder/` and `lsb_internal` prove compile-fail on external import by design.

Operation-style matrix (raw / strict-JPEG / in-place / borrowed-view / prepared / framed / tiled) with function tables: skill `stegoeggo-conventions` § Generic carrier API; per-family details: `carrier-lsb.md`, `carrier-jpeg.md`.
