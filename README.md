# stegoeggo

A modular Rust library for protecting images from AI scraping through steganographic watermarking and metadata injection for legal deterrence.

[![CI](https://github.com/yourorg/stegoeggo/actions/workflows/ci.yml/badge.svg)](https://github.com/yourorg/stegoeggo/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/stegoeggo)](https://crates.io/crates/stegoeggo)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## What is stegoeggo?

stegoeggo implements **image protection through steganographic watermarking and metadata injection** — techniques to protect images from being used to train AI models without the owner's consent. When AI systems scrape images from the web, protected images can:

- Carry visible metadata markers (XMP, IPTC DMI, EXIF) that serve as legal deterrents
- Contain hidden steganographic payloads (best-effort evidence channel; see [Robustness & Survival](#robustness--survival))
- Embed legal metadata (copyright, usage terms) directly into the image file
- Survive casual modification while retaining protection evidence

The library provides multiple **layers of protection** that work together:

| Layer | Description |
|-------|-------------|
| **Metadata Injection** | Embeds anti-scraping markers in image headers using XMP, IPTC DMI, and EXIF |
| **Steganography** | Hidden payloads embedded in image pixels (LSB) or DCT coefficients (JPEG) for verification |

### External Standards

- **IPTC Photo Metadata Standard** - Uses the DMI (Data Mining Inhibitor) tags from the [IPTC Photo Metadata Standard](https://iptc.org/standards/photo-metadata/) to communicate AI training restrictions
- **ISCC** - Computes [Immutable Self-Certifying Constituent Content](https://iscc-project.github.io/) identifiers for content identification

## Installation

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
stegoeggo = "0.2"
image = "0.25"  # Required for DynamicImage
```

For async support (Tokio-based WAF/CDN deployments):

```toml
[dependencies]
stegoeggo = { version = "0.2", features = ["async"] }
```

### As a CLI Tool

Build the binary from source:

```bash
cargo build --release
```

Or install directly:

```bash
cargo install stegoeggo
```

## Quick Start

### CLI

```bash
# Protect an image with default settings (Standard level)
stegoeggo input.png -o output.png

# Specify protection level
stegoeggo input.png -o output.png --level light

# With cryptographic key for HMAC-verified payloads
stegoeggo input.png -o output.png --key deadbeef123456

# With legal metadata (for content you own!)
stegoeggo input.png -o output.png --legal-claims

# Verify if an image is protected
stegoeggo protected.png -V
```

### Library

```rust
use stegoeggo::{ProtectionPipeline, ProtectionContext, ProtectionLevel};
use image::DynamicImage;

// Create pipeline and context
let pipeline = ProtectionPipeline::new();
let ctx = ProtectionContext::default();

// Process an image
let img = DynamicImage::new_rgb8(512, 512);
let protected = pipeline.process(&img, ProtectionLevel::Standard, &ctx).unwrap();
```

## Library Usage

### Processing Image Bytes

Process images from files or network sources without loading into DynamicImage:

```rust
use stegoeggo::{process_image_bytes, ProtectionContext, ProtectionLevel};

// Read image from file
let img_bytes = std::fs::read("image.png")?;

// Process with automatic format detection
let ctx = ProtectionContext::default();
let protected = process_image_bytes(&img_bytes, ProtectionLevel::Standard, &ctx)?;
```

### Parallel Processing

Process multiple images concurrently using Rayon:

```rust
use stegoeggo::{process_images_parallel, ProtectionContext, ProtectionLevel};
use image::DynamicImage;

let images: Vec<DynamicImage> = vec![
    image::open("image1.png")?,
    image::open("image2.png")?,
    image::open("image3.png")?,
];

let ctx = ProtectionContext::default();
let results = process_images_parallel(&images, ProtectionLevel::Standard, &ctx)?;
```

Or process bytes in parallel:

```rust
use stegoeggo::{process_images_bytes_parallel, ProtectionContext, ProtectionLevel};

let image_bytes: Vec<Vec<u8>> = vec![
    std::fs::read("image1.png")?,
    std::fs::read("image2.png")?,
];

let protected = process_images_bytes_parallel(&image_bytes, ProtectionLevel::Standard, &ctx)?;
```

### Protection Levels

The library provides three protection levels:

| Level | Strategy | Latency | Use Case |
|-------|----------|---------|----------|
| `Disabled` | No protection | <0.1ms | Testing, whitelisted clients |
| `Light` | Metadata + minimal stego (Q-table seed for JPEG, LSB redundancy=1 for PNG/WebP) | ~1-2ms (for 256x256; scales with image size) | Minimal visible markers, low cost |
| `Standard` | Full stego (DCT F5 + metadata for JPEG, LSB + metadata for PNG/WebP) | ~3-6ms | Default for most endpoints |

```rust
use stegoeggo::ProtectionLevel;

// Use different levels
let level = ProtectionLevel::Light;    // Metadata only
let level = ProtectionLevel::Standard; // Stego + Metadata (default)
```

### Cryptographic Key Support

Provide a hex key for keyed HMAC-SHA256 payload verification:

```rust
use stegoeggo::{ProtectionContext, ProtectionLevel};

// With MAC key - steganographic payloads are cryptographically verified
let key = vec![0xde, 0xad, 0xbe, 0xef, 0x12, 0x34, 0x56, 0x78];
let ctx = ProtectionContext::new(0.8, 42)
    .with_mac_key(key);

// Without key - same seed produces same output (checksum-based verification)
let ctx = ProtectionContext::new(0.8, 42);
```

> **Note:** `ProtectionContext::default()` uses `generate_random_seed()`, which is backed by the OS CSPRNG via the `getrandom` crate. The seed is unpredictable by design. For **reproducible** protection across runs, pass an explicit seed via `ProtectionContext::new(intensity, seed)`. In rare sandboxed environments where `getrandom` is unavailable, a time-based fallback is used and a warning is logged.

> **Security Notice:** Without a MAC key, steganographic payloads use a non-cryptographic
> CRC32 checksum that can be trivially forged by anyone who reads the source code. For
> production deployments (CDN protection, adversarial settings), **always** set a MAC key
> via `.with_mac_key()`. The default configuration without a key is suitable for
> development and testing only.

The MAC key affects:
- Steganography payload verification (HMAC-SHA256 instead of simple checksum)

### Legal Metadata Injection

Inject real legal metadata (copyright, contact info, usage terms). **Only use for content you own.**

Both `with_legal_metadata(...)` (provides the content) and `with_legal_claims(true)` (enables injection) are required — metadata will not be injected without both:

```rust
use stegoeggo::{ProtectionContext, LegalMetadata, ProtectionLevel};

let ctx = ProtectionContext::default()
    .with_legal_metadata(
        LegalMetadata::new()
            .with_copyright_holder("Your Company Name")
            .with_contact_email("legal@company.com")
            .with_usage_terms("All Rights Reserved. No AI training permitted.")
            .with_license_url("https://company.com/license")
    )
    .with_legal_claims(true);

let protected = process_image_bytes(&img_bytes, ProtectionLevel::Standard, &ctx)?;
```

### Granular Control

Control individual protection components:

```rust
use stegoeggo::{ProtectionContext, ProtectionLevel};

// Minimal - stego only, no metadata
let ctx = ProtectionContext::new(0.5, 42)
    .with_metadata_injection(false);

// Full - metadata + legal claims (for owned content)
let ctx = ProtectionContext::new(0.5, 42)
    .with_legal_claims(true)
    .with_legal_metadata(LegalMetadata::new()
        .with_copyright_holder("My Company"));

// Limit maximum image dimension for processing
let ctx = ProtectionContext::new(0.5, 42)
    .with_max_dimension(2048);
```

### DMI (Data Mining Inhibitor) Values

Set IPTC-standard DMI metadata values:

```rust
use stegoeggo::{ProtectionContext, DmiValue, ProtectionLevel};

let ctx = ProtectionContext::default()
    .with_dmi(DmiValue::ProhibitedAiMlTraining);
```

Available values:
- `Unspecified` - No restriction specified
- `Allowed` - Content may be used for AI/ML training
- `ProhibitedAiMlTraining` - Prohibited for AI/ML training
- `ProhibitedGenAiMlTraining` - Prohibited for generative AI training
- `ProhibitedExceptSearchEngineIndexing` - Prohibited except for search indexing
- `Prohibited` - All uses prohibited
- `ProhibitedSeeConstraints` - Prohibited, see constraints for details

### Performance Tuning

For latency-sensitive deployments:

```rust
use stegoeggo::{
    process_image_bytes_with_warnings, ImageOutputFormat, ProtectionContext, ProtectionLevel,
};

// Optimized context for WAF edge deployment
let ctx = ProtectionContext::new(0.5, seed)
    .with_format(ImageOutputFormat::Png)      // or Jpeg for smaller files
    .with_mac_key(mac_key)                     // required for adversarial serving
    .with_stego_redundancy(2)                        // 1-10, lower = faster
    .with_jpeg_quality(85)                         // 1-100, lower = faster
    .with_progressive_jpeg(true);                   // Progressive rendering for web

// Process and serve directly
let (protected_bytes, warnings) =
    process_image_bytes_with_warnings(&input_bytes, ProtectionLevel::Standard, &ctx)?;

// Reverse proxies should log warnings and may enforce policy before serving.
for warning in warnings {
    tracing::warn!(%warning, "stegoeggo protection warning");
}
```

**Configuration Guide:**

| Parameter | Default | Range | Effect on Latency |
|----------|---------|-------|-------------------|
| `stego_redundancy` | derived | 1-10 | Higher = more robust verification, slower. Default: derived from `intensity` (1 below 0.3, 2 from 0.3 to 0.7, 3 above) |
| `jpeg_quality` | 90 | 1-100 | Higher = larger files, same speed |
| `progressive_jpeg` | false | bool | Progressive = faster perceived load |
| `output_format` | PNG | PNG/JPEG/WebP | JPEG = smallest files |

### Reverse Proxy Integration Contract

`stegoeggo` owns steganographic embedding and metadata injection. The reverse proxy
should own cache lookup/storage, request byte limits, concurrency limits,
timeouts, and serving policy.

Recommended hot-path shape:

```rust
use stegoeggo::{
    process_image_bytes_with_warnings, ImageOutputFormat, ProtectionContext, ProtectionLevel,
    ProtectionWarning,
};

let ctx = ProtectionContext::new(0.5, seed)
    .with_format(ImageOutputFormat::Png)
    .with_mac_key(mac_key)
    .with_max_dimension(4096)
    .with_stego_redundancy(1);

let (protected, warnings) =
    process_image_bytes_with_warnings(&origin_bytes, ProtectionLevel::Standard, &ctx)?;

if warnings.iter().any(|w| matches!(w, ProtectionWarning::MissingMacKey)) {
    // Production policy should normally reject this configuration.
}
```

Use `process_image_bytes_with_warnings()` rather than the `DynamicImage` API in
the proxy path. For JPEG-in/JPEG-out, this keeps protection on the byte/DCT fast
path. For PNG/WebP, the library must still decode and re-encode pixels to embed
LSB payloads, so cache protected outputs aggressively at the proxy layer.

For verification, prefer `verify_image_bytes_detailed()`. A
`VerificationResult::MetadataOnly` result means metadata was found, but no
steganographic payload was integrity-verified; treat that as weaker evidence
than `VerificationResult::Verified`.

## CLI Usage

### Full Options Reference

```bash
stegoeggo [OPTIONS] <INPUT>

Arguments:
  <INPUT>                  Input image file(s)

Options:
  -o, --output <OUTPUT>    Output directory (batch) or file (single)
  -V, --verify             Verify if image contains protection signature
  -l, --level <LEVEL>      Protection level: disabled, light, standard
  -i, --intensity <FLOAT> Protection intensity 0.0-1.0 (default: 0.5)
  -s, --seed <SEED>        Seed for reproducible results
  -f, --format <FORMAT>   Output format: png, jpg, webp
  --stego-redundancy <N>  Stego redundancy 1-10 (default: 2). Higher = robust, lower = fast
  --jpeg-quality <N>       JPEG quality 1-100 (default: 90)
  --progressive            Use progressive JPEG encoding
  -v, --verbose            Print verbose output
  -d, --dmi <DMI>          DMI metadata value
  --metadata               Inject metadata (seed, DMI). Default: true for Standard
  --legal-claims          Inject legal claims (copyright). WARNING: only for content you own
  -k, --key <KEY>          Cryptographic key (hex string) for HMAC-SHA256 verification
  -j, --jobs <N>           Parallel jobs for batch processing (default: 1)
  -h, --help               Print help
  --version                Print version
```

### Examples

```bash
# Basic protection with default settings
stegoeggo photo.jpg -o photo_protected.png

# Light protection (metadata only)
stegoeggo art.png -o art_protected.png --level light

# With custom intensity and seed
stegoeggo image.jpg -o output.png -i 0.8 -s 12345

# Convert format while protecting
stegoeggo image.png -o image.jpg -f jpg

# WAF-optimized: fast processing with progressive JPEG
stegoeggo image.png -o image.jpg -f jpg --stego-redundancy 1 --jpeg-quality 85 --progressive

# WAF-optimized: PNG output, minimal latency
stegoeggo image.png -o protected.png --stego-redundancy 1

# With legal metadata
stegoeggo my_art.png -o protected.png --legal-claims --level standard

# With cryptographic key
stegoeggo image.png -o output.png --key a1b2c3d4e5f6

# Verify protection
stegoeggo output.png -V
```

### Verification

Check if an image has been protected:

```bash
stegoeggo image.png -V
```

Output examples:

```
# Protected image
Protected: Yes
Level: standard (id: 2)
Seed: 1234567890
Intensity: 0.50
Version: 1

# Unprotected image
Protected: No
This image does not contain a protection signature.
```

## How It Works

### 1. Metadata Injection

The library injects metadata into image headers:

**PNG:** tEXt and iTXt chunks
- `X-Protection-Seed`: Unique identifier for reproducibility
- `DMI-PROHIBITED`: IPTC DMI tag value
- Copyright/Contact/License: When legal claims enabled

**JPEG:** Comment markers and XMP packets
- COM markers for text metadata
- APP1 XMP packets for IPTC DMI tags

**WebP:** EXIF and XML chunks
- Similar metadata injection

### 2. Steganography

Hidden payloads embedded in images for verification and proof of protection:

**PNG/WebP:** LSB (Least Significant Bit) embedding
- Payload embedded in the lowest bits of RGB channels
- Redundant passes for verification robustness
- Uses pseudo-random pixel selection based on seed

**JPEG:** DCT-based (F5-style) embedding
- Seed embedded in quantization tables when those tables are preserved
- DCT coefficient perturbation using F5-style no-zero variant
- Pixel-domain JPEG fallback removed; JPEG protection now goes through the DCT fast path

**Payload Structure:**

The embedded payload has two variants depending on whether a MAC key is configured:

*MAC mode (32 bytes, with `with_mac_key`):*
```
Offset  Size  Field
0       1     Version (1)
1       1     Protection level
2       8     Seed (little-endian)
10      2     Intensity (0-100, little-endian)
12      8     Timestamp (Unix epoch)
20      4     Reserved/padding
24      8     HMAC-SHA256 (truncated to 8 bytes)
```

*Default mode (76 bytes, no MAC key — uses ECC for error recovery):*
```
Offset  Size  Field
0       24    Header (same as MAC mode above, bytes 0-23)
24      72    Reed-Solomon-like 3× repetition ECC encoding of the 24-byte header
96      4     CRC32 checksum of bytes 0-95
```

Without a MAC key, the payload uses 3× repetition coding with majority-vote decoding (`src/protected/ecc.rs`) so it can recover from bit corruption. With a MAC key, the 8-byte truncated HMAC-SHA256 provides cryptographic integrity.

## Integration Architecture

### Architecture Overview

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Image Source  │────▶│   Protection    │────▶│   Distribution  │
└─────────────────┘     │   Pipeline      │     └─────────────────┘
                        └─────────────────┘
                                │
                                │  1. Embed steganographic watermark
                                │  2. Inject metadata markers
                                │  3. Add legal claims (optional)
```

## Verification

### Programmatic Verification

```rust
use stegoeggo::{SteganographyProtector, MetadataTrapProtector};
use image::DynamicImage;

let img = image::load_from_memory(&protected_bytes)?;

// Method 1: Steganography verification
let stego = SteganographyProtector::new();
if stego.verify_payload(&img) {
    println!("Image is protected by stegoeggo");

    // Extract payload details
    if let Some(payload) = stego.extract_payload(&img) {
        println!("Protection level: {}", payload.protection_level());
        println!("Seed: {}", payload.seed());
        println!("Intensity: {:.2}", payload.intensity());
        println!("Version: {}", payload.version());
    }
}

// Method 2: Extract seed from metadata
let seed = MetadataTrapProtector::extract_seed_from_image(&protected_bytes);
if let Some(seed) = seed {
    println!("Found protection seed: {}", seed);
}
```

### JPEG Limitations

JPEG's lossy compression can destroy steganography payloads embedded in pixel data. This is an inherent limitation of the JPEG format and cannot be fully avoided.

**Current behavior:**
- PNG/WebP: LSB steganography is fully supported and verifiable
- JPEG: F5-style DCT steganography stores a seed in quantization tables when those tables are preserved and embeds payload bits in coefficients

**Recommendations:**
- Use PNG output format for protected images when possible
- For JPEG, a quantization-table seed is detection only; full verification relies on DCT payload integrity or metadata
- The library automatically uses the best available extraction method
- The CLI handles this and reports accordingly

**Technical note:** The library uses F5-style DCT embedding for JPEG. The quantization-table seed is useful when tables are preserved, but generic JPEG re-encoding can regenerate those tables and lose the seed.

## Robustness & Survival

Different protection layers survive different image transformations. The truth, verified by the test suite in `tests/robustness.rs` and `tests/robust_stego_matrix` (in `tests/robustness.rs`):

### What survives common transformations

| Transformation | Visible metadata (DMI, XMP, EXIF, COM) | Q-table seed (JPEG) | LSB stego payload (PNG/WebP) | DCT stego payload (JPEG) |
|----------------|-----------------------------------------|---------------------|------------------------------|--------------------------|
| **File copy / re-hosting** | ✓ | ✓ | ✓ | ✓ |
| **PNG ↔ PNG re-encode** | ✓ | n/a | ✓ (spread-spectrum + ECC + majority vote) | n/a |
| **WebP lossless ↔ WebP lossless** | ✓ | n/a | ✓ (same as PNG) | n/a |
| **WebP lossy (any re-encode)** | ✓ | n/a | ✗ (lossy codec destroys LSBs) | n/a |
| **JPEG → JPEG via `image` crate encoder** | ✗ (encoder strips COM/APP1) | ✗ (encoder rebuilds Q-tables) | ✗ (decoded to pixels) | ✗ |
| **JPEG → JPEG via `stegoeggo` fast path** | ✓ (re-injected) | ✓ (re-injected) | n/a | ✓ (DCT coeffs preserved) |
| **Format conversion (PNG ↔ JPEG) via `image` crate** | ✗ | ✗ | ✗ | ✗ |
| **Format conversion (WebP ↔ JPEG) via `image` crate** | ✗ | n/a | ✗ | n/a |
| **Crop** | ✗ (clipped) | ✗ | ✓ with `with_tile_size()` (≥1 intact tile) | partial (tile-aligned crops without re-encode) |
| **Resize** | ✗ (resampled) | ✗ | ✗ | ✗ |
| **Naive metadata strip** | ✗ | n/a | ✓ (still extractable) | partial |
| **LSB-preserving noise** (e.g. contrast, brightness) | ✓ | n/a | ✓ | n/a |
| **LSB-flipping noise** (e.g. random LSB overwrites) | ✓ | n/a | ✗ without ECC / partial with ECC | n/a |

### Encoder reality check

The `image` crate (and most general-purpose JPEG encoders) **do not preserve** COM or APP1 markers, and **rebuild standard Q-tables from scratch** on every encode. This means the visible metadata channel and the Q-table seed channel are both single-encoding only when the image passes through a generic encoder. The `stegoeggo` custom transcoder (`JpegTranscoder`) preserves DCT coefficients and re-injects metadata, but only when the image is processed through `process_image_bytes` (not through an external re-encoder).

### WebP caveat

`stegoeggo` uses LSB embedding for WebP, which only survives **lossless** WebP round-trips. The `image` crate's `WebPEncoder::new_lossless` preserves LSBs; lossy WebP re-encoding (the common web delivery path) destroys the LSB payload. If you serve protected WebP, configure your CDN to deliver lossless WebP, or convert protected output to PNG/JPEG-in-WebP-container with a tool that preserves the bitstream.

### Recommendations

- **For maximum legal evidence**: Use PNG output. The visible metadata + LSB stego payload survive almost everything except cropping, resizing, and re-encoding through a non-`stegoeggo` JPEG encoder. For crop resistance, add `.with_tile_size(64)` to the protection context — this embeds the payload in every 64×64 tile so any crop containing at least one full tile is recoverable.
- **For CDN/WAF deployment**: Use `Standard` level with PNG output. JPEG output discards the LSB payload and visible metadata on every re-compression.
- **For maximum robustness against stripping**: Set a MAC key via `with_mac_key()`. Without it, the embedded checksum can be trivially forged by anyone who reads this source.
- **For the strongest claims about evidence**: Serve the protected image directly and reference its ISCC code. Don't rely on downstream consumers to preserve any of the embedded channels.

### Honest threat model

The primary deterrence mechanism is **visible metadata injection** — DMI tags, TDM reservation, copyright, and structured COM markers. These are detectable by IPTC/XMP-aware scrapers and provide the strongest legal evidence *when preserved*. The steganographic payload is a **bonus evidence channel**: useful for proving the image was processed by this library at the point of distribution, but it is not designed to survive re-encoding through a general-purpose image pipeline. The library is a deterrent, not a forensic watermark.

## Performance

Benchmarked on Apple M1 Pro (10 cores), version 0.2.0:

| Image Size | Level | Time (ms) | Notes |
|------------|-------|-----------|-------|
| 256×256 | Light | ~1.0 | Metadata + minimal stego (LSB redundancy=1 or Q-table seed) |
| 256×256 | Standard | ~1.5 | Default settings |
| 256×256 | Standard | ~1.0 | `stego_redundancy=1` |
| 512×512 | Standard | ~5.0 | Default settings |
| 512×512 | Standard | ~3.0 | `stego_redundancy=1` |
| 1024×1024 | Standard | ~20.0 | Default settings |

**Target:** <10ms for typical image sizes

### Optimizations Applied (v0.2.0)

- Configurable stego redundancy (default 2x, can reduce to 1x)
- Pre-allocated buffers to reduce memory allocations
- Bounded fallback in steganography embedding
- Fast-path bytes processing without unnecessary re-encoding
- ISCC computation removed from hot path (available out-of-band)

## Technical Details

### Image Format Support

| Format | Metadata | Stego |
|--------|----------|-------|
| PNG | tEXt/iTXt | LSB |
| JPEG | COM/XMP/EXIF | DCT (F5) |
| WebP | EXIF/XML | LSB |

### ISCC Computation

The library computes ISCC-**like** (Immutable Self-Certifying Constituent Content) identifiers for content identification. **Note:** these identifiers are not guaranteed to be interoperable with the standard ISCC specification — they use a custom DCT-based perceptual hash and SHA-256 instance code. They are suitable for in-application deduplication and provenance tracking, but should not be used for cross-ISCC-tool interoperability:

```rust
use stegoeggo::{compute_iscc, Iscc};

let img = image::open("image.png")?;
let iscc = compute_iscc(&img);

println!("Content Code: {}", iscc.content);
println!("Data Code: {}", iscc.data);
println!("Instance Code: {}", iscc.instance);
println!("Full ISCC: {}", iscc.full);
```

The `Iscc` struct fields:
- `meta` — optional metadata code (not set by default)
- `content` — content-derived identifier (DCT-based perceptual hash)
- `data` — data-derived identifier (raw file hash)
- `instance` — identical to `data` (per-file identifier)
- `full` — full ISCC URI (e.g., `ISCC:...`)

### Error Handling

The library uses `thiserror` for error handling:

```rust
use stegoeggo::{Error, Result};

fn process() -> Result<DynamicImage> {
    // Operations that may fail
}
```

Common errors:
- `Error::ImageDecode(String)` - Failed to decode image
- `Error::ImageEncode(String)` - Failed to encode image
- `Error::Metadata(String)` - Metadata injection failure

## External References

- [IPTC Photo Metadata Standard](https://iptc.org/standards/photo-metadata/) - DMI tag specification
- [ISCC Project](https://iscc-project.github.io/) - Content identification standard
- [F5 Steganography](https://en.wikipedia.org/wiki/Steganography#Embedding) - DCT-based steganographic technique
- [jpeg-encoder](https://crates.io/crates/jpeg-encoder) - JPEG encoding used
- [image crate](https://image.rs/) - Image processing foundation

## Architecture

```
stegoeggo
├── ProtectionPipeline        # Main orchestration
├── Protector trait           # Strategy pattern for protectors
│   ├── PassthroughProtector      # No-op (Disabled level)
│   ├── MetadataTrapProtector     # Metadata injection (always)
│   └── SteganographyProtector    # LSB/DCT embedding (Light: minimal, Standard: full)
├── ProtectionLevel          # disabled → light → standard
├── LegalMetadata            # Configurable legal metadata
├── ProtectionContext        # Configuration for protection
└── StegoPayload             # Extracted stego data
```

**Steganography intensity by level:**
- `Disabled`: none
- `Light`: minimal — Q-table seed (JPEG) or LSB redundancy=1 (PNG/WebP)
- `Standard`: full — DCT F5 (JPEG) or LSB + ECC + spread-spectrum (PNG/WebP)

## Safety & Ethics

This library is designed to protect intellectual property from unauthorized AI training. It is intended for:

- Protecting personal photos from being scraped
- Defending artist portfolios from model training
- Securing proprietary images on CDNs
- Content owners who have not licensed their work for AI training

**We do not endorse:**
- Using this library for malicious purposes
- Circumventing legitimate AI services' terms of service
- Poisoning images you do not own or have rights to
- Any use that violates applicable laws

This is a defensive tool for content protection, not an offensive weapon against AI systems.

**Production use requires MAC keys:** Without a cryptographic MAC key, steganographic payloads use a non-cryptographic CRC32 checksum that can be forged. For adversarial or production deployments (e.g., CDN protection), always set a MAC key via `.with_mac_key()` to ensure payload integrity.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please ensure:

1. Tests pass: `cargo test`
2. Code is formatted: `cargo fmt`
3. No clippy warnings: `cargo clippy`
