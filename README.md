# stegoeggo

Embed rights-reservation metadata and AI-training restriction notices into images, with optional best-effort steganographic markers for redundant evidence.

[![CI](https://github.com/eggstack/stegoeggo/actions/workflows/ci.yml/badge.svg)](https://github.com/eggstack/stegoeggo/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/stegoeggo)](https://crates.io/crates/stegoeggo)
[![Documentation](https://docs.rs/stegoeggo/badge.svg)](https://docs.rs/stegoeggo)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.87-blue.svg)](https://blog.rust-lang.org/)

## What stegoeggo is

stegoeggo is:

- A **legal-notice and rights-reservation metadata tool** for images.
- A way to make copyright and AI-training restrictions visible to metadata-aware systems.
- A best-effort redundant marking system when optional steganographic payloads are enabled.

## What stegoeggo is not

stegoeggo is **not**:

- A forensic watermarking system.
- A DRM system.
- A guarantee that marks survive arbitrary resizing, re-encoding, screenshots, cropping, or metadata stripping.
- A cryptographic proof that a model trained on a specific image.
- A data-poisoning tool.

## What it does

stegoeggo embeds multiple layers of rights-reservation and AI-training restriction metadata into images:

| Layer | Description |
|-------|-------------|
| **Metadata Injection** | Embeds rights-reservation and AI-training restriction markers in image headers using canonical `plus:DataMining` rights signals, XMP, and EXIF |
| **Steganography** | Optional hidden payloads embedded in image pixels (LSB) or DCT coefficients (JPEG) for redundant evidence |

### External Standards

- **PLUS License Data Format** - Emits `plus:DataMining` with official PLUS LDF controlled-vocabulary URIs for machine-readable rights signals (canonical per the [PLUS License Data Format](https://www.useplus.com/) specification). Legacy `Iptc4xmpExt:DMI-*` properties are still parsed for backward compatibility but not emitted by default.
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
cargo install stegoeggo-cli
```

## Quick Start

### CLI

```bash
# Embed legal-notice metadata with default settings (Standard level)
stegoeggo input.png -o output.png

# With explicit legal metadata (recommended for owned content)
stegoeggo artwork.png -o artwork_protected.png \
  --copyright-holder "Jane Artist" \
  --creator "Jane Artist" \
  --rights-url "https://example.com/rights/artwork" \
  --no-genai-training

# With full legal metadata including new v0.3.0 fields
stegoeggo photo.jpg --copyright-holder "Acme Corp" --creator "Jane Doe" \
  --credit-line "Photo by Jane Doe / Acme Corp" \
  --copyright-owner "Acme Corp" \
  --licensor-name "Acme Corp" --licensor-email "legal@acme.com" \
  --content-created-at "2024-01-15"

# Quick AI-training restriction
stegoeggo photo.jpg -o protected.jpg --no-ai-training

# Light protection (metadata only, minimal stego)
stegoeggo input.png -o output.png --level light

# Authenticated provenance (optional — requires MAC key)
stegoeggo artwork.png -o artwork_auth.png \
  --profile authenticated-provenance \
  --key deadbeefcafebabe \
  --copyright-holder "Jane Artist" \
  --rights-url "https://example.com/rights/artwork" \
  --no-ai-training

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

// Process an image — embeds metadata and optional steganographic markers
let img = DynamicImage::new_rgb8(512, 512);
let protected = pipeline.process(&img, ProtectionLevel::Standard, &ctx).unwrap();
```

### Request-Based API (recommended for new code)

```rust
use stegoeggo::{ProtectionRequest, RightsPolicy, RightsNotice, LegalMetadata};

let request = ProtectionRequest::metadata_only(
    RightsNotice::default(),
    RightsPolicy::ProhibitedAiMlTraining,
)
.with_legal_metadata(
    LegalMetadata::new()
        .with_copyright_holder("Example Corp")
        .with_usage_terms("All Rights Reserved"),
);

let protected = stegoeggo::process_request_bytes(&img_bytes, &request)?;
```

## Library Usage

### Processing Image Bytes

Process images from files or network sources without loading into DynamicImage:

```rust,no_run
use stegoeggo::{process_image_bytes, ProtectionContext, ProtectionLevel};

// Read image from file
let img_bytes = std::fs::read("image.png").unwrap();

// Process with automatic format detection. The byte API preserves the detected
// input format unless you set `ProtectionContext::with_format(...)`.
let ctx = ProtectionContext::default();
let protected = process_image_bytes(&img_bytes, ProtectionLevel::Standard, &ctx).unwrap();
```

### Parallel Processing

Process multiple images concurrently using Rayon:

```rust,no_run
use stegoeggo::{process_images_parallel, ProtectionContext, ProtectionLevel};
use image::DynamicImage;

let images: Vec<DynamicImage> = vec![
    image::open("image1.png").unwrap(),
    image::open("image2.png").unwrap(),
    image::open("image3.png").unwrap(),
];

let ctx = ProtectionContext::default();
let results = process_images_parallel(&images, ProtectionLevel::Standard, &ctx).unwrap();
```

Or process bytes in parallel:

```rust,ignore
use stegoeggo::{process_images_bytes_parallel, ProtectionContext, ProtectionLevel};

let image_bytes: Vec<Vec<u8>> = vec![
    std::fs::read("image1.png").unwrap(),
    std::fs::read("image2.png").unwrap(),
];

let protected = process_images_bytes_parallel(&image_bytes, ProtectionLevel::Standard, &ctx).unwrap();
```

### Request-Based API (Recommended)

The request-based API is the canonical way to use stegoeggo. It separates
rights policy from processing mechanics:

```rust
use stegoeggo::{ProtectionRequest, RightsPolicy, ProtectionPreset, RightsNotice};

// Metadata-only legal notice (fastest path)
let request = ProtectionRequest::metadata_only(
    RightsNotice::default(),
    RightsPolicy::ProhibitedAiMlTraining,
);

// With hidden marker
let request = ProtectionRequest::with_hidden_marker(
    RightsNotice::default(),
    RightsPolicy::ProhibitedAiMlTraining,
);

// Using a preset
let request = ProtectionRequest::from_preset(
    ProtectionPreset::AuthenticatedProvenance,
    RightsNotice::default(),
    RightsPolicy::ProhibitedAiMlTraining,
)
.with_mac_key(b"secret".to_vec());

let (protected, report) = stegoeggo::process_request_bytes_with_report(&img_bytes, &request)?;
println!("Metadata injected: {}", report.metadata_injected);
println!("Stego succeeded: {}", report.stego_succeeded);
```

### Protection Levels

The library provides three protection levels:

| Level | Strategy | Latency (512x512) | Use Case |
|-------|----------|-------------------|----------|
| `Disabled` | No protection | ~20 ns | Testing, whitelisted clients |
| `Light` | Metadata + minimal stego (Q-table seed for JPEG, LSB redundancy=1 for PNG/WebP) | ~0.8 ms | Metadata-only, low cost |
| `Standard` | Full stego (DCT F5 + metadata for JPEG, LSB + metadata for PNG/WebP) | ~0.8 ms | Default for most endpoints |

```rust
use stegoeggo::ProtectionLevel;

// Use different levels
let level = ProtectionLevel::Light;    // Metadata + minimal stego
let level = ProtectionLevel::Standard; // Stego + Metadata (default)
```

### Evidence Profiles

Evidence profiles control how protection warnings are interpreted and the default evidence posture. While `ProtectionLevel` controls how much processing occurs, `EvidenceProfile` answers "what evidence model is the caller trying to express?"

| Profile | MAC Key Required | Stego | Primary Use Case |
|---------|-----------------|-------|------------------|
| `LegalNotice` (default) | No | Optional | Standards-aligned metadata notice |
| `LegalNoticeWithStego` | No | Yes | Metadata notice plus best-effort hidden marker |
| `AuthenticatedProvenance` | Yes | Yes | Cryptographic proof of payload origin |
| `Maximal` | Optional | Yes | All available evidence channels |

```rust
use stegoeggo::{ProtectionContext, EvidenceProfile, LegalMetadata, ProtectionLevel};

// Legal notice only — no MAC key needed
let ctx = ProtectionContext::legal_notice()
    .with_legal_metadata(
        LegalMetadata::new()
            .with_copyright_holder("Jane Artist")
            .with_ai_constraints("No AI training permitted.")
    );

// Authenticated provenance — MAC key expected
let ctx = ProtectionContext::authenticated_provenance()
    .with_mac_key(b"secret-key".to_vec());

// Via builder
let ctx = ProtectionContext::new(0.5, 42)
    .with_evidence_profile(EvidenceProfile::Maximal);
```

### Legal Metadata Injection

Inject real legal metadata (copyright, contact info, usage terms). **Only use for content you own.**

`with_legal_metadata(...)` provides the content. When legal metadata is provided, `with_legal_claims(true)` is **auto-enabled** — you no longer need to call it explicitly. The explicit call is still supported but no longer required:

```rust,ignore
use stegoeggo::{process_image_bytes, ProtectionContext, LegalMetadata, ProtectionLevel};

let img_bytes = std::fs::read("image.png").unwrap();
let ctx = ProtectionContext::default()
    .with_legal_metadata(
        LegalMetadata::new()
            .with_copyright_holder("Your Company Name")
            .with_contact_email("legal@company.com")
            .with_usage_terms("All Rights Reserved. No AI training permitted.")
            .with_license_url("https://company.com/license")
    )
    .with_legal_claims(true);

let protected = process_image_bytes(&img_bytes, ProtectionLevel::Standard, &ctx).unwrap();
```

### DMI (Data Mining Inhibitor) Values

Set DMI metadata values for AI-training restrictions. The XMP writer emits canonical `plus:DataMining` properties with PLUS LDF vocabulary keys. Legacy `Iptc4xmpExt:DMI-*` properties are still parsed for backward compatibility but not emitted by default:

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

Each variant maps to a canonical PLUS vocabulary key via `DmiValue::plus_vocab_key()` (e.g., `DMI-PROHIBITED-AIMLTRAINING`). Legacy IPTC keys can be parsed back via `DmiValue::from_plus_vocab_key()`.

### TDMRep Status

TDMRep (W3C Text and Data Mining Reservation Protocol) deployment artifacts (HTTP headers, `/.well-known/tdmrep.json`) are **deferred** from Release 1. StegoEggo currently emits PLUS image metadata only. Legacy `tdm:reserve_tdm` image properties are still parsed for backward compatibility diagnostics but are not emitted by default. The CLI `--tdm-reserved` flag is deprecated and now sets DMI to `ProhibitedSeeConstraints`.

### Optional: Authenticated Stego Provenance (MAC Key)

Provide a hex key for HMAC-SHA256 payload verification. Without a key, steganographic payloads use a non-cryptographic CRC32 checksum suitable for development and testing.

```rust
use stegoeggo::{ProtectionContext, ProtectionLevel};

// With MAC key — steganographic payloads are cryptographically verified
let key = vec![0xde, 0xad, 0xbe, 0xef, 0x12, 0x34, 0x56, 0x78];
let ctx = ProtectionContext::new(0.8, 42)
    .with_mac_key(key);

// Without key — same seed produces same output (checksum-based verification)
let ctx = ProtectionContext::new(0.8, 42);
```

> **Note:** `ProtectionContext::default()` uses `generate_random_seed()`, which is backed by the OS CSPRNG via the `getrandom` crate. The seed is unpredictable by design. For **reproducible** protection across runs, pass an explicit seed via `ProtectionContext::new(intensity, seed)`. In rare sandboxed environments where `getrandom` is unavailable, a time-based fallback is used and a warning is logged.

**Verification profiles:**

- **Without a MAC key** (legal-notice mode): Steganographic payload verification uses a non-cryptographic CRC32 checksum with ECC redundancy. Visible metadata markers prove intent and rights reservation. No MAC key is required for the legal-notice use case.
- **With a MAC key** (authenticated provenance mode): The library uses HMAC-SHA256 for cryptographic payload verification. This proves the hidden payload was generated by a party with the configured secret. Use this when you need cryptographic integrity for the steganographic channel.

The MAC key affects:
- Steganography payload verification (HMAC-SHA256 instead of simple checksum)

### Migration from ProtectionLevel API

The `ProtectionLevel` and `EvidenceProfile` APIs still work but are deprecated.
To migrate:

| Old API | New API |
|---------|---------|
| `process_image_bytes(&bytes, ProtectionLevel::Standard, &ctx)` | `process_request_bytes(&bytes, &request)` |
| `ctx.with_dmi(DmiValue::ProhibitedAiMlTraining)` | `RightsPolicy::ProhibitedAiMlTraining` in `ProtectionRequest` |
| `EvidenceProfile::LegalNotice` | `ProtectionPreset::LegalNotice` or `ProtectionChannels::metadata_only()` |
| `ctx.with_metadata_injection(false)` | `ProtectionChannels { rights_metadata: false, .. }` |

### Granular Control

Control individual protection components:

```rust
use stegoeggo::{LegalMetadata, ProtectionContext, ProtectionLevel};

// Minimal - stego only, no metadata
let ctx = ProtectionContext::new(0.5, 42)
    .with_metadata_injection(false);

// Full - metadata + legal claims (for owned content)
// Legal claims are auto-enabled when LegalMetadata is provided,
// but you can still pass `true` explicitly if desired.
let ctx = ProtectionContext::new(0.5, 42)
    .with_legal_metadata(LegalMetadata::new()
        .with_copyright_holder("My Company"));

// Limit maximum image dimension for processing
let ctx = ProtectionContext::new(0.5, 42)
    .with_max_dimension(2048);
```

### Performance Tuning

For latency-sensitive deployments:

```rust,ignore
use stegoeggo::{
    process_image_bytes_with_warnings, ImageOutputFormat, ProtectionContext, ProtectionLevel,
};

// Optimized context for WAF edge deployment
let seed = 42u64;
let mac_key = b"your-secret-key".to_vec();
let input_bytes = std::fs::read("image.png").unwrap();
let ctx = ProtectionContext::new(0.5, seed)
    .with_format(ImageOutputFormat::Png)      // or Jpeg for smaller files
    .with_mac_key(mac_key)                     // for authenticated provenance
    .with_stego_redundancy(2)                  // 1-10, lower = faster
    .with_jpeg_quality(85)                     // 1-100, lower = faster
    .with_progressive_jpeg(true);              // Progressive rendering for web

// Process and serve directly
let (protected_bytes, warnings) =
    process_image_bytes_with_warnings(&input_bytes, ProtectionLevel::Standard, &ctx).unwrap();

// Reverse proxies should log warnings and may enforce policy before serving.
for warning in &warnings {
    eprintln!("Warning: {warning}");
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

```rust,ignore
use stegoeggo::{
    process_image_bytes_with_warnings, ImageOutputFormat, ProtectionContext, ProtectionLevel,
    ProtectionWarning,
};

let seed = 42u64;
let mac_key = b"your-secret-key".to_vec();
let origin_bytes = std::fs::read("image.png").unwrap();
let ctx = ProtectionContext::new(0.5, seed)
    .with_format(ImageOutputFormat::Png)
    .with_mac_key(mac_key)
    .with_max_dimension(4096)
    .with_stego_redundancy(1);

let (protected, warnings) =
    process_image_bytes_with_warnings(&origin_bytes, ProtectionLevel::Standard, &ctx).unwrap();

if warnings.iter().any(|w| matches!(w, ProtectionWarning::MissingMacKey)) {
    // Production policy should normally reject this configuration.
}
```

Use `severity_for_profile()` to determine if a warning is actionable for your evidence model:

```rust,ignore
use stegoeggo::{process_image_bytes_with_warnings, EvidenceProfile, ProtectionContext, ProtectionLevel};

let profile = EvidenceProfile::AuthenticatedProvenance;
let (protected, warnings) =
    process_image_bytes_with_warnings(&origin_bytes, ProtectionLevel::Standard, &ctx).unwrap();

for w in &warnings {
    match w.severity_for_profile(profile) {
        WarningSeverity::Error => eprintln!("FATAL: {w}"),
        WarningSeverity::Warning => eprintln!("WARN: {w}"),
        WarningSeverity::Info => {} // silently ignored
    }
}
```

Use `process_image_bytes_with_warnings()` rather than the `DynamicImage` API in
the proxy path. For JPEG-in/JPEG-out, this keeps protection on the byte/DCT fast
path. For PNG/WebP, the library must still decode and re-encode pixels to embed
LSB payloads, so cache protected outputs aggressively at the proxy layer.

For verification, prefer `verify_legal_notice()` for a comprehensive report of all
evidence channels. It extracts legal notice fields (copyright, creator, contact, etc.),
checks steganographic payload integrity, and returns an `EvidenceStrength` rating.
Use `verify_image_bytes_detailed()` for lower-level payload-only verification.

## CLI Usage

### Full Options Reference

```bash
stegoeggo [OPTIONS] <INPUT>

Arguments:
  <INPUT>                  Input image file(s)

Options:
  -o, --output <OUTPUT>    Output directory (batch) or file (single)
  -V, --verify             Verify legal-notice report, evidence strength, and channels
  -l, --level <LEVEL>      Protection level: disabled, light, standard
  -p, --profile <PROFILE>  Evidence profile: legal-notice, legal-notice-stego,
                           authenticated-provenance, maximal (default: legal-notice)
  -i, --intensity <FLOAT> Protection intensity 0.0-1.0 (default: 0.5)
  -s, --seed <SEED>        Seed for reproducible results
  -f, --format <FORMAT>   Output format: png, jpg, webp (default: png)
  --stego-redundancy <N>  Stego redundancy 1-10 (default: 2). Higher = robust, lower = fast
  --jpeg-quality <N>       JPEG quality 1-100 (default: 90)
  --progressive            Use progressive JPEG encoding
  -v, --verbose            Print verbose output
  -d, --dmi <DMI>          AI-training restriction metadata (DMI value; emitted as canonical plus:DataMining)
  --metadata               Inject metadata (seed, DMI). Default: true for Light and Standard
  --legal-claims          Inject legal claims (copyright, usage terms) — only for content you own
  --copyright-holder <NAME>  Copyright holder name (e.g., 'Jane Doe' or 'Acme Corp')
  --creator <NAME>        Creator/author name (e.g., 'Jane Doe')
  --contact <EMAIL_OR_URL>  Contact email or URL for rights inquiries
  --rights-url <URL>      URL to full usage terms or license text
  --usage-terms <TEXT>    Brief usage terms summary (e.g., 'All rights reserved')
  --ai-constraints <TEXT>  AI-specific constraints (e.g., 'No training, no generation')
  --no-ai-training        Shorthand: prohibit AI/ML training and set default AI constraints
  --no-genai-training     Shorthand: prohibit generative AI training only
  --tdm-reserved          [DEPRECATED] Sets DMI ProhibitedSeeConstraints (TDMRep deferred)
  -k, --key <KEY>          Optional cryptographic key (hex string) for HMAC-SHA256 verification
  -j, --jobs <N>           Parallel jobs for batch processing (default: 1)
  --strict                 Exit with error if any warnings have error severity for the active profile
  -h, --help               Print help
  --version                Print version
```

### Examples

```bash
# Basic protection with default settings
stegoeggo photo.jpg -o photo_protected.png

# Light protection (metadata + minimal stego)
stegoeggo art.png -o art_protected.png --level light

# With custom intensity and seed
stegoeggo image.jpg -o output.png -i 0.8 -s 12345

# Convert format while protecting
stegoeggo image.png -o image.jpg -f jpg

# WAF-optimized: fast processing with progressive JPEG
stegoeggo image.png -o image.jpg -f jpg --stego-redundancy 1 --jpeg-quality 85 --progressive

# WAF-optimized: PNG output, minimal latency
stegoeggo image.png -o protected.png --stego-redundancy 1

# With legal metadata (explicit claims)
stegoeggo my_art.png -o protected.png --legal-claims --level standard

# With full legal metadata — auto-enables legal claims (no --legal-claims needed)
stegoeggo my_art.png -o protected.png \
  --copyright-holder "Jane Doe" \
  --contact "jane@example.com" \
  --rights-url "https://example.com/license" \
  --usage-terms "All rights reserved" \
  --no-ai-training

# Quick AI-training restriction
stegoeggo photo.jpg -o protected.jpg --no-genai-training

# With cryptographic key for authenticated provenance
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
Version: 2

# Unprotected image
Protected: No
This image does not contain a protection signature.
```

## How It Works

### 1. Metadata Injection

The library injects rights-reservation and AI-training restriction metadata into image headers:

**PNG:** tEXt and iTXt chunks
- `X-Protection-Seed`: Unique identifier for reproducibility
- `plus:DataMining`: Canonical PLUS LDF DMI value (e.g., `DMI-PROHIBITED-AIMLTRAINING`)
- Copyright/Contact/License: When legal claims enabled

**JPEG:** Comment markers and XMP packets
- COM markers for text metadata
- APP1 XMP packets with `plus:DataMining` (canonical) and legacy `Iptc4xmpExt:DMI-*` (parsed only)

**WebP:** EXIF and XML chunks
- Similar metadata injection with XMP-based DMI

### 2. Steganography (Optional)

Hidden payloads embedded in images for redundant verification evidence:

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

*MAC mode (40 bytes, with `with_mac_key`):*
```
Offset  Size  Field
0       1     Version (2)
1       1     Protection level
2       8     Seed (little-endian)
10      2     Intensity (0-100, little-endian)
12      8     Timestamp (Unix epoch)
20      4     Content hash (truncated ISCC or SHA-256)
24      1     DMI value
25      1     Flags (reserved)
26      6     Reserved/padding
32      8     HMAC-SHA256 (truncated to 8 bytes)
```

*Default mode (100 bytes, no MAC key — uses ECC for error recovery):*
```
Offset  Size  Field
0       96    Reed-Solomon-like 3x repetition ECC encoding of the 32-byte header
96      4     CRC32 checksum of bytes 0-95
```

Without a MAC key, the payload uses 3x repetition coding with majority-vote decoding (`src/protected/ecc.rs`) so it can recover from bit corruption. With a MAC key, the 8-byte truncated HMAC-SHA256 provides cryptographic integrity.

## Integration Architecture

### Architecture Overview

```
+-----------------+     +-----------------+     +-----------------+
|   Image Source  |---->|   Protection    |---->|   Distribution  |
+-----------------+     |   Pipeline      |     +-----------------+
                        +-----------------+
                                |
                                |  1. Inject metadata markers (primary)
                                |  2. Embed steganographic markers (redundant)
                                |  3. Add legal claims (optional)
```

## Verification

### Programmatic Verification

```rust,ignore
use stegoeggo::{SteganographyProtector, MetadataTrapProtector};
use image::DynamicImage;

let protected_bytes = std::fs::read("protected.png").unwrap();
let img = image::load_from_memory(&protected_bytes).unwrap();

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

// Method 3: Comprehensive legal notice verification (recommended)
let report = stegoeggo::verify_legal_notice(&protected_bytes, b"my-mac-key");
println!("Copyright holder: {:?}", report.copyright_holder());
println!("Evidence strength: {}", report.evidence_strength());
for channel in report.channels() {
    println!("  Channel: {}", channel);
}
```

#### Evidence Strength Levels

| Level | Meaning |
|-------|---------|
| `NoNoticeFound` | No metadata or steganographic markers detected |
| `MetadataNoticeOnly` | Legal notice metadata found, no stego payload verified |
| `MetadataNoticeAndBestEffortStego` | Metadata + unauthenticated stego payload verified |
| `MetadataNoticeAndAuthenticatedProvenance` | Metadata + MAC-authenticated stego payload verified |

#### Evidence Channels

The `NoticeVerification` report lists which evidence channels were detected:
`PngText`, `PngXmp`, `JpegComment`, `JpegXmp`, `JpegIptc`, `WebPXmp`, `WebPExif`, `LsbPayload`, `DctPayload`, `QTableSeed`.

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
| **File copy / re-hosting** | Yes | Yes | Yes | Yes |
| **PNG <-> PNG re-encode** | Yes | n/a | Yes (spread-spectrum + ECC + majority vote) | n/a |
| **WebP lossless <-> WebP lossless** | Yes | n/a | Yes (same as PNG) | n/a |
| **WebP lossy (any re-encode)** | Yes | n/a | No (lossy codec destroys LSBs) | n/a |
| **JPEG -> JPEG via `image` crate encoder** | No (encoder strips COM/APP1) | No (encoder rebuilds Q-tables) | No (decoded to pixels) | No |
| **JPEG -> JPEG via `stegoeggo` fast path** | Yes (re-injected) | Yes (re-injected) | n/a | Yes (DCT coeffs preserved) |
| **Format conversion (PNG <-> JPEG) via `image` crate** | No | No | No | No |
| **Format conversion (WebP <-> JPEG) via `image` crate** | No | n/a | No | n/a |
| **Crop** | No (clipped) | No | Yes with `with_tile_size()` (>=1 intact tile) | partial (tile-aligned crops without re-encode) |
| **Resize** | No (resampled) | No | No | No |
| **Naive metadata strip** | No | n/a | Yes (still extractable) | partial |
| **LSB-preserving noise** (e.g. contrast, brightness) | Yes | n/a | Yes | n/a |
| **LSB-flipping noise** (e.g. random LSB overwrites) | Yes | n/a | No without ECC / partial with ECC | n/a |

### Encoder reality check

The `image` crate (and most general-purpose JPEG encoders) **do not preserve** COM or APP1 markers, and **rebuild standard Q-tables from scratch** on every encode. This means the visible metadata channel and the Q-table seed channel are both single-encoding only when the image passes through a generic encoder. The `stegoeggo` custom transcoder (`JpegTranscoder`) preserves DCT coefficients and re-injects metadata, but only when the image is processed through `process_image_bytes` (not through an external re-encoder).

### WebP caveat

`stegoeggo` uses LSB embedding for WebP, which only survives **lossless** WebP round-trips. The `image` crate's `WebPEncoder::new_lossless` preserves LSBs; lossy WebP re-encoding (the common web delivery path) destroys the LSB payload. If you serve protected WebP, configure your CDN to deliver lossless WebP, or convert protected output to PNG/JPEG-in-WebP-container with a tool that preserves the bitstream.

### Recommendations

- **For maximum legal evidence**: Use PNG output. The visible metadata + LSB stego payload survive almost everything except cropping, resizing, and re-encoding through a non-`stegoeggo` JPEG encoder. For crop resistance, add `.with_tile_size(64)` to the protection context — this embeds the payload in every 64x64 tile so any crop containing at least one full tile is recoverable.
- **For CDN/WAF deployment**: Use `Standard` level with PNG output. JPEG output discards the LSB payload and visible metadata on every re-compression.
- **For authenticated provenance**: Set a MAC key via `with_mac_key()` to cryptographically sign steganographic payloads.
- **For the strongest claims about evidence**: Serve the protected image directly and reference its ISCC code. Don't rely on downstream consumers to preserve any of the embedded channels.

### Honest threat model

The primary deterrence mechanism is **visible metadata injection** — canonical `plus:DataMining` rights signals, copyright, and structured COM markers. These are detectable by PLUS/XMP-aware scrapers and provide the strongest legal evidence *when preserved*. The steganographic payload is a **bonus evidence channel**: useful for proving the image was processed by this library at the point of distribution, but it is not designed to survive re-encoding through a general-purpose image pipeline. The library is a deterrent, not a forensic watermark.

## Performance

Benchmarked on Apple M4 Pro (12 cores), version 0.2.0.

### In-Memory Processing (`DynamicImage` path)

| Image Size | Light | Standard |
|------------|-------|----------|
| 256x256 | 0.2 ms | 0.2 ms |
| 512x512 | 0.8 ms | 0.8 ms |
| 1024x1024 | 3.2 ms | 3.1 ms |
| 2560x2560 (2K) | 18 ms | 20 ms |
| 3840x3840 (4K) | 35 ms | 40 ms |

### Bytes-in/Bytes-out Processing (production path for WAF/CDN)

PNG in / PNG out — the "maximum legal evidence" path:

| Image Size | Light | Standard |
|------------|-------|----------|
| 512x512 | 0.7 ms | 0.7 ms |
| 2560x2560 (2K) | 11 ms | 13 ms |
| 3840x3840 (4K) | 25 ms | 29 ms |

### JPEG Fast Path

JPEG-in / JPEG-out bypasses pixel decode entirely and operates directly on DCT coefficients:

| Image Size | Time |
|------------|------|
| 256x256 | **1.3 us** |
| 512x512 | 1.6 ms |

### Tiled Embedding (crop-resistant mode)

JPEG with `with_tile_size(64)`:

| Image Size | Embed | Extract |
|------------|-------|---------|
| 256x256 | 1.5 ms | 270 ms |
| 1024x1024 | 253 ms | — |

### Allocations

Standard protection at 512x512: 60 allocations, 5.7 MB peak.

### Summary

- **<1 ms** for images up to 512x512
- **<5 ms** for images up to 1024x1024
- **<30 ms** for 4K images (bytes path, Standard level)
- JPEG fast path is sub-millisecond for small images

## Technical Details

### Image Format Support

| Format | Metadata | Stego |
|--------|----------|-------|
| PNG | tEXt/iTXt | LSB |
| JPEG | COM/XMP/EXIF | DCT (F5) |
| WebP | EXIF/XML | LSB |

### ISCC Computation

The library computes ISCC-**like** (Immutable Self-Certifying Constituent Content) identifiers for content identification. **Note:** these identifiers are not guaranteed to be interoperable with the standard ISCC specification — they use a custom DCT-based perceptual hash and SHA-256 instance code. They are suitable for in-application deduplication and provenance tracking, but should not be used for cross-ISCC-tool interoperability:

```rust,ignore
use stegoeggo::{compute_iscc, Iscc};

let img = image::open("image.png").unwrap();
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

```rust,ignore
use stegoeggo::{Error, Result};

fn process() -> Result<image::DynamicImage> {
    // Operations that may fail
}
```

Common errors:
- `Error::ImageDecode(String)` - Failed to decode image
- `Error::ImageEncode(String)` - Failed to encode image
- `Error::Metadata(String)` - Metadata injection failure

## External References

- [PLUS License Data Format](https://www.useplus.com/) - Canonical rights metadata standard (PLUS LDF controlled-vocabulary URIs)
- [IPTC Photo Metadata Standard](https://iptc.org/standards/photo-metadata/) - Legacy DMI tag specification (still parsed for backward compatibility)
- [ISCC Project](https://iscc-project.github.io/) - Content identification standard
- [F5 Steganography](https://en.wikipedia.org/wiki/Steganography#Embedding) - DCT-based steganographic technique
- [jpeg-encoder](https://crates.io/crates/jpeg-encoder) - JPEG encoding used
- [image crate](https://image.rs/) - Image processing foundation

## Legal Notice Model

See [docs/legal_notice_model.md](docs/legal_notice_model.md) for a detailed description of the legal notice and evidence model, including what metadata channels are embedded, what transformations commonly remove notices, and operational recommendations.

## External Metadata Conformance

The conformance suite validates that protected images expose correct
rights metadata to external tools. It uses a layered approach:

1. **Fixture manifest** — machine-readable TOML manifest with SHA-256 digests, expected values, and provenance
2. **Manifest validation** — structural checks (duplicate IDs, path traversal, invalid formats/categories, SHA-256 validity) run before any fixtures are processed
3. **Internal extraction** — `verify_legal_notice()` parses the image
4. **External extraction** — ExifTool extracts metadata independently
5. **Namespace-aware XMP validation** — xmllint validates XML structure
6. **Normalized comparison** — internal and external results are compared field-by-field
7. **Coverage enforcement** — strict mode requires explicit per-category and per-format minimums
8. **Machine-readable report** — JSON output with per-check pass/fail/warn

Strict mode requires `--manifest` and evaluates per-fixture expectations from the manifest. The harness returns stable exit codes (0–5) for scripting.

### Running Conformance Checks

```bash
# Build the conformance harness
cargo build --release --bin stegoeggo-conformance

# Run all fixtures with manifest verification (requires exiftool + xmllint)
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict \
  --json report.json

# Or use the shell wrapper (checks for all required tools)
./scripts/verify_metadata_conformance.sh --strict --json report.json
```

### Expected Field Visibility by Format

| Field | PNG (tEXt/XMP) | JPEG (COM/XMP) | WebP (XMP) |
|-------|-----------------|-----------------|-------------|
| Copyright | `Copyright` | `Comment: Copyright (c) ...` | `dc:rights` |
| Creator | `Creator` | `Comment: Creator: ...` | `dc:creator` |
| Usage Terms | `UsageTerms` | `Comment: UsageTerms: ...` | `xmpRights:UsageTerms` |
| Rights URL | `WebStatement` | `Comment: WebStatement: ...` | `xmpRights:WebStatement` |
| AI Constraints | `AIConstraints` | `Comment: AIConstraints: ...` | `stegoeggo:AIConstraints` |
| DMI Policy | `XMP-plus:DataMining` | `XMP-plus:DataMining` | `XMP-plus:DataMining` |

### Caveats

- ExifTool is the authoritative external parser. Other tools may not expose
  all XMP properties depending on namespace support.
- PNG tEXt `XML:com.adobe.xmp` requires ExifTool to decode — plain `xmllint`
  cannot extract XMP from PNG containers.
- JPEG COM markers are stegoeggo-specific and may not be visible in all tools.
- WebP XMP visibility depends on the tool's support for `dc:rights`,
  `dc:creator`, `xmpRights:*`, and `stegoeggo:*` namespaces.

### What Conformance Does and Does Not Prove

**Proves:**
- Protected images expose correct rights metadata to external parsers (ExifTool)
- XMP is well-formed and namespace-correct
- Internal extraction matches external extraction field-by-field
- Metadata survives re-processing (idempotence)
- Unrelated metadata is preserved through the update path
- Format writers produce semantically equivalent metadata (PNG vs JPEG vs WebP)

**Does not prove:**
- Legal enforceability of embedded rights statements
- That all external tools will parse every XMP namespace
- That metadata survives arbitrary transformations (social media re-encoding, screenshots, aggressive cropping)
- That steganographic payloads survive lossy compression
- Compliance with any specific legal jurisdiction

### Installing External Tools

The conformance suite requires `exiftool` and `xmllint`.

**macOS (Homebrew):**
```bash
brew install exiftool libxml2
```

**Ubuntu/Debian:**
```bash
sudo apt-get install libimage-exiftool-perl libxml2-utils
```

**Fedora/RHEL:**
```bash
sudo dnf install perl-Image-ExifTool libxml2
```

**Arch Linux:**
```bash
sudo pacman -S perl-image-exiftool libxml2
```

### Adding Fixtures

1. Place the image in the appropriate `tests/fixtures/conformance/<category>/` directory
2. Add an entry to `tests/fixtures/conformance/manifest.toml` with provenance, SHA-256 digest, and expected values
3. Document provenance in `tests/fixtures/conformance/README.md`
4. Verify: `cargo run --bin stegoeggo-conformance -- --fixtures tests/fixtures/conformance --manifest tests/fixtures/conformance/manifest.toml --strict`

## Contributor Checklist

Before submitting a change that affects metadata output:

- [ ] Canonical writer test updated
- [ ] Legacy reader test preserved
- [ ] External fixture added or reviewed (`tests/fixtures/conformance/`)
- [ ] Namespace-aware validation passes
- [ ] Cross-format matrix passes (PNG, JPEG, WebP)
- [ ] Preservation/idempotence passes
- [ ] Strict external conformance passes (`./scripts/verify_metadata_conformance.sh --strict`)

## Architecture

```
stegoeggo
+-- ProtectionPipeline        # Main orchestration
+-- Protector trait           # Strategy pattern for protectors
|   +-- PassthroughProtector      # No-op (Disabled level)
|   +-- MetadataTrapProtector     # Metadata injection (always)
|   +-- SteganographyProtector    # LSB/DCT embedding (Light: minimal, Standard: full)
+-- ProtectionLevel          # disabled -> light -> standard
+-- LegalMetadata            # Configurable legal metadata
+-- ProtectionContext        # Configuration for protection
+-- StegoPayload             # Extracted stego data
```

**Steganography intensity by level:**
- `Disabled`: none
- `Light`: minimal — Q-table seed (JPEG) or LSB redundancy=1 (PNG/WebP)
- `Standard`: full — DCT F5 (JPEG) or LSB + ECC + spread-spectrum (PNG/WebP)

## Safety & Ethics

This library uses `#![forbid(unsafe_code)]` throughout — no `unsafe` blocks exist in the library crate. All image processing is built on safe Rust with the `image` crate.

This library is designed to protect intellectual property from unauthorized AI training. It is intended for:

- Protecting personal photos from being scraped
- Defending artist portfolios from model training
- Securing proprietary images on CDNs
- Content owners who have not licensed their work for AI training

**We do not endorse:**
- Using this library for malicious purposes
- Circumventing legitimate AI services' terms of service
- Applying restrictions to images you do not own or have rights to
- Any use that violates applicable laws

This is a defensive tool for content protection, not an offensive weapon against AI systems.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please ensure:

1. Tests pass: `cargo test --all-features`
2. Code is formatted: `cargo fmt --check`
3. No clippy warnings: `cargo clippy --all-targets --all-features -- -D warnings`
4. Package builds: `cargo package --workspace --allow-dirty`
