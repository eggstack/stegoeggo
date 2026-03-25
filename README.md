# cloakrs

A modular Rust library for protecting images from AI scraping through adversarial poisoning strategies. Designed for CDN/WAF edge deployment with sub-10ms latency.

[![CI](https://github.com/yourorg/cloakrs/actions/workflows/ci.yml/badge.svg)](https://github.com/yourorg/cloakrs/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/cloakrs)](https://crates.io/crates/cloakrs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## What is cloakrs?

cloakrs implements **adversarial image protection** - a technique to protect images from being used to train AI models without the owner's consent. When AI systems scrape images from the web, poisoned images can:

- Degrade model quality when included in training datasets
- Introduce visual artifacts in generated outputs
- Make trained models unreliable for certain inputs
- Serve as a deterrent to unauthorized scraping

The library provides multiple **layers of protection** that work together:

| Layer | Description |
|-------|-------------|
| **Metadata Injection** | Embeds anti-scraping markers in image headers using IPTC DMI standard |
| **Adversarial Perturbation** | Adds imperceptible noise patterns that disrupt AI model training |
| **Steganography** | Hidden payloads embedded in image pixels for verification |
| **Precomputed Variants** | Cached perturbations for ultra-fast CDN edge deployment |

### External Standards

- **IPTC Photo Metadata Standard** - Uses the DMI (Data Mining Inhibitor) tags from the [IPTC Photo Metadata Standard](https://iptc.org/standards/photo-metadata/) to communicate AI training restrictions
- **ISCC** - Computes [Immutable Self-Certifying Constituent Content](https://iscc-project.github.io/) identifiers for content identification

## Installation

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
cloakrs = "0.2"
image = "0.25"  # Required for DynamicImage
```

### As a CLI Tool

Build the binary from source:

```bash
cargo build --release
```

Or install directly:

```bash
cargo install cloakrs
```

## Quick Start

### CLI

```bash
# Protect an image with default settings (Standard level)
cloakrs input.png -o output.png

# Specify protection level
cloakrs input.png -o output.png --level strong

# With cryptographic key for unique perturbations
cloakrs input.png -o output.png --key deadbeef123456

# With legal metadata (for content you own!)
cloakrs input.png -o output.png --legal-claims

# Verify if an image is protected
cloakrs protected.png -V
```

### Library

```rust
use cloakrs::{ProtectionPipeline, ProtectionContext, ProtectionLevel};
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
use cloakrs::{process_image_bytes, ProtectionContext, ProtectionLevel};

// Read image from file
let img_bytes = std::fs::read("image.png")?;

// Process with automatic format detection
let ctx = ProtectionContext::default();
let protected = process_image_bytes(&img_bytes, ProtectionLevel::Standard, &ctx)?;
```

### Parallel Processing

Process multiple images concurrently using Rayon:

```rust
use cloakrs::{process_images_parallel, ProtectionContext, ProtectionLevel};
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
use cloakrs::{process_images_bytes_parallel, ProtectionContext, ProtectionLevel};

let image_bytes: Vec<Vec<u8>> = vec![
    std::fs::read("image1.png")?,
    std::fs::read("image2.png")?,
];

let protected = process_images_bytes_parallel(&image_bytes, ProtectionLevel::Standard, &ctx)?;
```

### Protection Levels

The library provides five protection levels:

| Level | Strategy | Latency | Use Case |
|-------|----------|---------|----------|
| `Disabled` | No protection | <0.1ms | Testing, whitelisted clients |
| `Light` | Metadata injection only | ~2ms | Minimal visible markers |
| `Standard` | Noise + Stego + Metadata | ~3-6ms | Default for most endpoints |
| `Enhanced` | Enhanced perturbation | ~5-8ms | Higher protection needs |
| `Strong` | Precomputed variants | ~2-6ms | High-value content, CDN |

```rust
use cloakrs::ProtectionLevel;

// Use different levels
let level = ProtectionLevel::Light;    // Metadata only
let level = ProtectionLevel::Standard;  // Balanced (default)
let level = ProtectionLevel::Enhanced; // Stronger noise
let level = ProtectionLevel::Strong;    // Precomputed for speed
```

### Cryptographic Key Support

Provide a hex key for keyed perturbations that make output unique and non-reproducible without the key:

```rust
use cloakrs::{ProtectionContext, ProtectionLevel};

// With MAC key - perturbations are cryptographically keyed
let key = vec![0xde, 0xad, 0xbe, 0xef, 0x12, 0x34, 0x56, 0x78];
let ctx = ProtectionContext::new(0.8, 42)
    .with_mac_key(key);

// Without key - same seed produces same output
let ctx = ProtectionContext::new(0.8, 42);
```

The MAC key affects:
- Noise pattern generation
- Steganography payload verification (HMAC instead of simple checksum)

### Legal Metadata Injection

Inject real legal metadata (copyright, contact info, usage terms). **Only use for content you own:**

```rust
use cloakrs::{ProtectionContext, LegalMetadata, ProtectionLevel};

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
use cloakrs::{ProtectionContext, ProtectionLevel};

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
use cloakrs::{ProtectionContext, DmiValue, ProtectionLevel};

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

### WAF Edge Optimization

For CDN/WAF edge deployment with sub-10ms latency requirements:

```rust
use cloakrs::{process_image_bytes, ProtectionContext, ProtectionLevel, ImageOutputFormat};

// Optimized context for WAF edge deployment
let ctx = ProtectionContext::new(0.5, seed)
    .with_output_format(ImageOutputFormat::Png)      // or Jpeg for smaller files
    .with_stego_redundancy(2)                        // 1-5, lower = faster
    .with_jpeg_quality(85)                         // 1-100, lower = faster
    .with_progressive_jpeg(true);                   // Progressive rendering for web

// Process and serve directly
let protected_bytes = process_image_bytes(&input_bytes, ProtectionLevel::Standard, &ctx)?;
```

**Configuration Guide:**

| Parameter | Default | Range | Effect on Latency |
|----------|---------|-------|-------------------|
| `stego_redundancy` | 2 | 1-5 | Higher = more robust verification, slower |
| `jpeg_quality` | 90 | 1-100 | Higher = larger files, same speed |
| `progressive_jpeg` | false | bool | Progressive = faster perceived load |
| `output_format` | PNG | PNG/JPEG/WebP | JPEG = smallest files |

## CLI Usage

### Full Options Reference

```bash
cloakrs [OPTIONS] <INPUT>

Arguments:
  <INPUT>                  Input image file(s)

Options:
  -o, --output <OUTPUT>    Output directory (batch) or file (single)
  -V, --verify             Verify if image contains protection signature
  -l, --level <LEVEL>      Protection level: disabled, light, standard, enhanced, strong
  -t, --target <TARGET>   Target AI model: sd15, sd21, sdxl, dalle, midjourney
  -i, --intensity <FLOAT> Protection intensity 0.0-1.0 (default: 0.5)
  -s, --seed <SEED>        Seed for reproducible results
  -f, --format <FORMAT>   Output format: png, jpg, webp
  --stego-redundancy <N>  Stego redundancy 1-5 (default: 2). Higher = robust, lower = fast
  --jpeg-quality <N>       JPEG quality 1-100 (default: 90)
  --progressive            Use progressive JPEG encoding
  -v, --verbose            Print verbose output
  -d, --dmi <DMI>          DMI metadata value
  --metadata               Inject metadata (seed, DMI). Default: true for Standard+
  --legal-claims          Inject legal claims (copyright). WARNING: only for content you own
  -k, --key <KEY>          Cryptographic key (hex string) for keyed perturbations
  -j, --jobs <N>           Parallel jobs for batch processing (default: 1)
  -h, --help               Print help
  --version                Print version
```

### Examples

```bash
# Basic protection with default settings
cloakrs photo.jpg -o photo_protected.png

# Strong protection with custom target model
cloakrs art.png -o art_protected.png --level strong --target sdxl

# With custom intensity and seed
cloakrs image.jpg -o output.png -i 0.8 -s 12345

# Convert format while protecting
cloakrs image.png -o image.jpg -f jpg

# WAF-optimized: fast processing with progressive JPEG
cloakrs image.png -o image.jpg -f jpg --stego-redundancy 1 --jpeg-quality 85 --progressive

# WAF-optimized: PNG output, minimal latency
cloakrs image.png -o protected.png --stego-redundancy 1

# With legal metadata
cloakrs my_art.png -o protected.png --legal-claims --level standard

# With cryptographic key
cloakrs image.png -o output.png --key a1b2c3d4e5f6

# Verify protection
cloakrs output.png -V
```

### Verification

Check if an image has been protected:

```bash
cloakrs image.png -V
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

### 2. Adversarial Perturbation

Adds imperceptible noise in three stages:

1. **Block Noise Generation**: Creates block-based noise patterns using a seeded random generator
2. **Spatial Variation**: Applies brightness variations across horizontal strips
3. **Frequency Perturbation**: Adds sinusoidal patterns in the frequency domain

The noise intensity is controlled by the `intensity` parameter (0.0-1.0), with higher values producing more visible but more disruptive perturbations.

### 3. Steganography

Hidden payloads embedded in images:

**PNG/WebP:** LSB (Least Significant Bit) embedding
- Payload embedded in the lowest bits of RGB channels
- 2x redundancy for verification
- Uses pseudo-random pixel selection based on seed

**JPEG:** DCT-based or pixel-based embedding
- Block-structured embedding with redundancy
- Survives some re-encoding but less robust than PNG

**Payload Structure (32 bytes):**
```
Offset  Size  Field
0       1     Version (1)
1       1     Protection level
2       8     Seed (little-endian)
10      2     Intensity (0-100, little-endian)
12      8     Timestamp (Unix epoch)
20      4     Reserved/padding
24      8     HMAC (if key provided) or checksum
```

### 4. Precomputed Variants

For CDN/WAF edge deployment:

1. Precompute perturbations for known images
2. Store variants with cache keys (hash + target + level + intensity)
3. At edge, look up variant and apply instantly
4. Achieves sub-10ms latency for cached content

## CDN/WAF Integration

### Architecture Overview

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Origin Server │────▶│  CDN / WAF Edge │────▶│    End User     │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                        │
        │  1. Upload images     │                        │
        │  2. Precompute        │                        │
        │     variants         │                        │
        │                      │  3. Request            │
        │                      │  4. Lookup variant     │
        │                      │  5. Apply + serve      │
```

### Precomputation Workflow

```rust
use cloakrs::{
    ProtectionPipeline, ProtectionContext, ProtectedVariant, 
    ProtectionLevel
};
use image::DynamicImage;
use std::collections::HashMap;

// 1. Load original images (typically at upload time)
let original = image::open("original.png")?;
let hash = cloakrs::util::image::compute_image_hash(&original);

// 2. Generate perturbation data
let pipeline = ProtectionPipeline::new();
let ctx = ProtectionContext::new(0.5, 42);

// 3. Register precomputed variant
let (width, height) = original.dimensions();
let precomputed = pipeline.precomputed.clone();

let perturbation = precomputed.generate_perturbation_data(&original, &ctx)?;

let variant = ProtectedVariant::new(
    hash,
    ProtectionLevel::Strong,
    perturbation,
    0.5,
    width,
    height,
);

precomputed.register_variant(variant)?;

// 4. Store variant (serialize to JSON for CDN)
// In practice, store in Redis, database, or file
let json = serde_json::to_string(&variant)?;
std::fs::write("variant.json", json)?;
```

### Edge Application

At the CDN edge (using precomputed variants):

```rust
use cloakrs::{ProtectionPipeline, ProtectionContext, ProtectionLevel, ProtectedVariant};

// Load variant from storage (Redis, database, etc.)
let variant: ProtectedVariant = serde_json::from_str(&stored_json)?;

// Register the variant
let pipeline = ProtectionPipeline::new();
pipeline.register_precomputed_variants(vec![variant])?;

// Now requests hit the fast path
let img = image::open(requested_image)?;
let protected = pipeline.process(&img, ProtectionLevel::Strong, &ctx)?;
```

## Verification

### Programmatic Verification

```rust
use cloakrs::{SteganographyProtector, MetadataTrapProtector};
use image::DynamicImage;

let img = image::load_from_memory(&protected_bytes)?;

// Method 1: Steganography verification
let stego = SteganographyProtector::new();
if stego.verify_payload(&img) {
    println!("Image is protected by cloakrs");
    
    // Extract payload details
    if let Some(payload) = stego.extract_payload(&img) {
        println!("Protection level: {}", payload.protection_level);
        println!("Seed: {}", payload.seed);
        println!("Intensity: {}", payload.intensity);
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
- JPEG: Steganography is embedded using a pixel-based approach that may survive some re-encoding, but is not guaranteed

**Recommendations:**
- Use PNG output format for protected images when possible
- If you must use JPEG, verification relies on metadata (seed extraction)
- For highest reliability, use the DCT-based stego which embeds in quantization tables (survives re-encoding better)
- The CLI automatically handles this and reports accordingly

**Technical note:** The library attempts DCT-based embedding for JPEG which modifies quantization tables, providing better durability against re-encoding than pixel-based approaches.

## Performance

Benchmarked on Apple M1 Pro (10 cores), version 0.2.0:

| Image Size | Level | Time (ms) | Notes |
|------------|-------|-----------|-------|
| 256×256 | Light | ~0.1 | Metadata only |
| 256×256 | Standard | ~1.5 | Default settings |
| 256×256 | Standard | ~1.0 | `stego_redundancy=1` |
| 512×512 | Standard | ~5.0 | Default settings |
| 512×512 | Standard | ~3.0 | `stego_redundancy=1` |
| 1024×1024 | Standard | ~20.0 | Default settings |

**Target:** <10ms for CDN/WAF edge deployment

### Optimizations Applied (v0.2.0)

- Configurable stego redundancy (default 2x, can reduce to 1x)
- Pre-allocated buffers to reduce memory allocations
- Bounded fallback in steganography embedding
- Fast-path bytes processing without unnecessary re-encoding
- ISCC computation removed from hot path (available out-of-band)
- Precomputed sin tables for frequency perturbations

## Technical Details

### Image Format Support

| Format | Metadata | Stego | Perturbation |
|--------|----------|-------|---------------|
| PNG | tEXt/iTXt | LSB | Full |
| JPEG | COM/XMP | DCT/Pixel | Full |
| WebP | EXIF/XML | LSB | Full |

### ISCC Computation

The library computes ISCC (Immutable Self-Certifying Constituent Content) identifiers:

```rust
use cloakrs::{compute_iscc, Iscc};

let img = DynamicImage::open("image.png")?;
let iscc = compute_iscc(&img);

println!("Content Code: {}", iscc.content);
println!("Data Code: {}", iscc.data);
println!("Full ISCC: {}", iscc.full);
```

### Error Handling

The library uses `thiserror` for error handling:

```rust
use cloakrs::{Error, Result};

fn process() -> Result<DynamicImage> {
    // Operations that may fail
}
```

Common errors:
- `Error::ImageDecode(String)` - Failed to decode image
- `Error::ImageEncode(String)` - Failed to encode image  
- `Error::Metadata(String)` - Metadata injection failure
- `Error::VariantNotFound(String)` - Precomputed variant not found

## External References

- [IPTC Photo Metadata Standard](https://iptc.org/standards/photo-metadata/) - DMI tag specification
- [ISCC Project](https://iscc-project.github.io/) - Content identification standard
- [Adversarial ML Overview](https://adversarial-ml-tutorial.org/) - Background on adversarial perturbations
- [jpeg-encoder](https://crates.io/crates/jpeg-encoder) - JPEG encoding used
- [image crate](https://image.rs/) - Image processing foundation

## Architecture

```
cloakrs
├── ProtectionPipeline        # Main orchestration
├── Protector trait           # Strategy pattern for protectors
│   ├── PassthroughProtector # No-op (Disabled level)
│   ├── MetadataTrapProtector # Metadata injection (Light level)
│   ├── NoiseProtector        # Adversarial noise (Standard level)
│   ├── EnhancedProtector     # Enhanced noise (Enhanced level)
│   └── PrecomputedProtector  # Cached variants (Strong level)
├── SteganographyProtector   # LSB/DCT embedding
├── ProtectionLevel          # disabled → light → standard → enhanced → strong
├── LegalMetadata            # Configurable legal metadata
├── ProtectionContext            # Configuration for protection
└── ProtectedVariant         # Precomputed variant storage
```

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific module tests
cargo test poisoners
cargo test steganography
```

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

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please ensure:

1. Tests pass: `cargo test`
2. Code is formatted: `cargo fmt`
3. No clippy warnings: `cargo clippy`
