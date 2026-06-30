# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Fixed
- Reject JPEG encoding when image dimensions exceed the encoder's 16-bit JPEG limits instead of silently truncating width or height.
- Report LSB capacity warnings from the effective payload size, including the shorter MAC-protected payload form.
- Return the first warning from `process_image_bytes_with_info()` instead of filtering out non-progressive advisories.
- Keep `Disabled` byte processing as a byte-for-byte no-op, even for unrecognized input bytes.
- Return JPEG entropy transcoder errors instead of panicking on missing pre-built Huffman tables.

### Changed
- Reduced duplicated byte API format/context setup and centralized JPEG encoding.

## [0.2.0] - 2026-06-04

### Added
- **Crop-resistant tiled steganography** via `with_tile_size(n)` on `ProtectionContext`:
  - Embeds the full payload in each `tile_size × tile_size` tile independently
  - LSB tiled path for PNG/WebP; F5 tiled path for JPEG
  - Any crop containing ≥1 intact tile is recoverable
  - Configurable `tile_extraction_max_origins` to bound extraction time
- **Metadata injection** via `MetadataTrapProtector` — IPTC DMI tags, TDM reservation (ISO/IEC 21000-21), XMP packets, EXIF UserComment for AI-training opt-out markers
- **Steganographic watermarking** via `SteganographyProtector`:
  - LSB embedding for PNG/WebP with spread-spectrum, content-adaptive amplitude, and 3× repetition ECC
  - F5-style DCT coefficient embedding for JPEG with no-zero coefficient variant (avoids shrinkage detection)
  - Seed embedded in JPEG quantization tables (survives byte-preserving metadata insertion; not arbitrary recompression)
- **Three protection levels**: `Disabled`, `Light`, `Standard`
- **Verification API** (`verify_image_bytes`, `verify_payload`, `extract_payload`) with optional HMAC-SHA256 key verification
- **ISCC-like content identification** via `compute_iscc`
- **Async API** (behind `async` feature) for WAF/CDN edge integration
- **Parallel batch processing** via Rayon
- **JPEG fast path** (`JpegTranscoder`) operating on DCT coefficients without pixel decode/encode
- **Legal metadata injection** for copyright, contact, license, and usage terms
- **CLI tool** (`stegoeggo`) with batch processing and format conversion

### Changed
- MSRV: 1.87
- Byte APIs preserve detected input format by default; the CLI defaults to PNG output unless `--format` is set
- Stego redundancy now 1-10 (was 1-5)
- Default stego redundancy derived from intensity: 1 (<0.3), 2 (0.3-0.7), 3 (≥0.7)

### Fixed
- JPEG segment length bounds handling in header parser
- Runtime check for non-8-multiple bit lengths in `bits_to_bytes`
- JPEG redundancy embedding bug (embedded = 0 reset between passes)
- Dimension validation in `process_bytes` for both JPEG and non-JPEG paths

## [0.1.0] - 2026-03-13

Initial release.

[Unreleased]: https://github.com/eggstack/stegoeggo/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/eggstack/stegoeggo/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/eggstack/stegoeggo/releases/tag/v0.1.0
