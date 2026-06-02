# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2024-06-02

### Added
- **Metadata injection** via `MetadataTrapProtector` — IPTC DMI tags, TDM reservation (ISO/IEC 21000-21), XMP packets, EXIF UserComment for AI-training opt-out markers
- **Steganographic watermarking** via `SteganographyProtector`:
  - LSB embedding for PNG/WebP with spread-spectrum, content-adaptive amplitude, and 3× repetition ECC
  - F5-style DCT coefficient embedding for JPEG with no-zero coefficient variant (avoids shrinkage detection)
  - Seed embedded in JPEG quantization tables (survives re-encoding)
- **Three protection levels**: `Disabled`, `Light`, `Standard`
- **Verification API** (`verify_image_bytes`, `verify_payload`, `extract_payload`) with optional HMAC-SHA256 key verification
- **ISCC-like content identification** via `compute_iscc`
- **Async API** (behind `async` feature) for WAF/CDN edge integration
- **Parallel batch processing** via Rayon
- **JPEG fast path** (`JpegTranscoder`) operating on DCT coefficients without pixel decode/encode
- **Legal metadata injection** for copyright, contact, license, and usage terms
- **CLI tool** (`cloakrs`) with batch processing and format conversion

### Changed
- MSRV: 1.87
- Default output format changed from format-conversion to PNG preservation
- Stego redundancy now 1-10 (was 1-5)
- Default stego redundancy derived from intensity: 1 (<0.3), 2 (0.3-0.7), 3 (≥0.7)

### Fixed
- JPEG segment length bounds handling in header parser
- Runtime check for non-8-multiple bit lengths in `bits_to_bytes`
- JPEG redundancy embedding bug (embedded = 0 reset between passes)
- Dimension validation in `process_bytes` for both JPEG and non-JPEG paths

## [0.1.0] - 2024-03-13

Initial release.