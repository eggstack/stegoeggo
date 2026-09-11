# WebP Container

**Source:** `src/webp_container.rs` (~2090 lines, `pub(crate)`) — strict RIFF parser and re-serializer for WebP injection.

Owns the `WEBP` RIFF model that `protected/metadata_trap/webp.rs` mutates and `container_walk.rs` accounts. All offsets are `usize` with checked arithmetic; malformed input returns `Error::Metadata` / `Error::InvalidFormat`, never panics.

## Types

| Type | Role |
|------|------|
| `ParsedWebP { data, image_kind, chunks, vp8x_index, xmp_indices, exif_indices, icc_indices, vp8_indices, vp8l_indices, anim_indices, anmf_indices, alph_indices, has_xmp, has_exif, has_icc, has_alpha, has_animation }` | Owned parse result; `data` is the working buffer that re-serialization rewrites |
| `RiffChunk { fourcc, data_start, data_len }` | Chunk directory entry; `fourcc_str()` renders non-UTF8 as `"????"` |
| `WebPImageKind::{LossyVP8, LosslessVP8L, ExtendedVP8X}` | Top-level image kind from the RIFF layout |
| `FrameImageKind::{LossyVP8, LosslessVP8L}` + `AnmfFrameInfo` | Per-animation-frame kind/geometry for `ANMF` frames |

## Parse rules

- Requires `RIFF` + `WEBP` magic, LE32 sizes, even-byte padding per chunk.
- Recognizes `VP8X`, `VP8 `, `VP8L`, `XMP `, `EXIF`, `ICCP`, `ALPH`, `ANIM`, `ANMF`.
- `VP8X` feature flags are re-derived on write via `derive_features(...).with_xmp(...)` and `encode_vp8x_chunk`, which clears reserved bits `0xC1`. Malformed inputs with those bits set are silently normalized to spec-compliant output — intentional, breaks byte-for-byte idempotence for malformed fixtures (see `overview.md` WebP correctness note).
- `ANMF` sub-frames are descended for kind/alpha discovery; unknown FourCCs are indexed, never rejected.

## Write path

Injection rebuilds the RIFF structure: insert/replace `XMP `/`EXIF` chunks, fix up the `VP8X` flags, update the RIFF size. There is no JPEG-style preserving-encode (DQT/SOS-only) equivalent — WebP always rewrites the container.

## Consumers

- `protected/metadata_trap/webp.rs` — XMP/EXIF injection and extraction.
- `protected/notice_verification.rs` — `extract_webp_notice`.
- `container_walk.rs` — accounting-only RIFF walk (lenient, no errors).
- Fuzz coverage: `webp_riff_parser` target (see `testing.md`).
