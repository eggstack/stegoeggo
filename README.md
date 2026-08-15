# stegoeggo

Embed machine-readable rights-reservation metadata and AI-training restriction notices in images, with optional best-effort steganographic markers for redundant evidence.

[![CI](https://github.com/eggstack/stegoeggo/actions/workflows/ci.yml/badge.svg)](https://github.com/eggstack/stegoeggo/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/stegoeggo)](https://crates.io/crates/stegoeggo)
[![Documentation](https://docs.rs/stegoeggo/badge.svg)](https://docs.rs/stegoeggo)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.87-blue.svg)](https://blog.rust-lang.org/)

`stegoeggo` is primarily a **rights-notice metadata tool**. It writes explicit rights policy and copyright information into PNG, JPEG, and WebP files. A hidden marker can also be added as a second, best-effort evidence channel.

It is not DRM, a forensic watermark, a data-poisoning system, or proof that a particular model trained on an image. Metadata can be stripped, and hidden markers can be damaged or removed by transformations such as screenshots, cropping, resizing, or re-encoding.

## Installation

Install the CLI from crates.io:

```bash
cargo install stegoeggo-cli
```

Or build the workspace CLI from source:

```bash
git clone https://github.com/eggstack/stegoeggo.git
cd stegoeggo
cargo build --release --bin stegoeggo
./target/release/stegoeggo --help
```

For the Rust library:

```toml
[dependencies]
stegoeggo = "0.3"
```

The minimum supported Rust version is **1.87**. CI currently exercises Linux x86_64; see [SUPPORT.md](SUPPORT.md) for the maintained platform and feature matrix.

## Quick start

For new scripts, prefer the policy-first CLI flags and make the rights policy explicit.

Write a metadata-only AI/ML training prohibition:

```bash
stegoeggo image.png -o image_protected.png \
  --rights-policy prohibited-ai-ml-training \
  --preset legal-notice \
  --copyright-notice "© 2026 Example Artist. All rights reserved." \
  --creator "Example Artist" \
  --rights-url "https://example.com/rights" \
  --usage-terms "No AI/ML training."
```

Add the best-effort hidden marker as a redundant channel:

```bash
stegoeggo image.png -o image_protected.png \
  --rights-policy prohibited-ai-ml-training \
  --preset legal-notice-with-stego
```

Inspect an existing file:

```bash
stegoeggo image_protected.png --verify
```

Machine-readable verification output is available with `--json`:

```bash
stegoeggo image_protected.png --verify --json
```

A directory can be processed as a batch. `-j` controls worker count:

```bash
stegoeggo ./images -o ./protected \
  --rights-policy prohibited-ai-ml-training \
  --preset legal-notice \
  -j 4
```

Input format is detected from the image data. Unless `--format` is supplied, the CLI preserves the input format. With no explicit output path, protected files use a `_protected` suffix.

### A note about the CLI defaults

The older `--level`/`--profile` interface remains for compatibility. A bare invocation such as:

```bash
stegoeggo image.png
```

uses the legacy `standard` default, which resolves to rights metadata, a best-effort hidden marker, and the `ProhibitedAiMlTraining` policy. New automation should use `--rights-policy` plus `--preset` explicitly so the requested legal policy and evidence channels are visible in the command itself.

The legacy `--dmi` and `--tdm-reserved` options are also retained for compatibility. Current output uses the canonical PLUS `plus:DataMining` signal; legacy DMI/TDM representations are read for verification but are not the preferred output interface.

## Rights policies

`--rights-policy` maps directly to the library's `RightsPolicy` enum and, when a policy is specified, to the corresponding PLUS License Data Format controlled-vocabulary value in `plus:DataMining` XMP metadata.

| CLI value | Meaning |
|---|---|
| `unspecified` | Do not emit a `plus:DataMining` policy value |
| `allowed` | Data mining allowed |
| `prohibited-ai-ml-training` | AI/ML training prohibited |
| `prohibited-generative-ai-training` | Generative-AI training prohibited |
| `prohibited-except-search-indexing` | Data mining prohibited except search-engine indexing |
| `prohibited-all-data-mining` | All data mining prohibited |
| `prohibited-see-constraints` | Prohibited; consult the supplied constraints |

The CLI also accepts convenience flags such as `--no-ai-training` and `--no-genai-training`, but `--rights-policy` is the clearest interface for new scripts. Contradictory policy options are rejected rather than silently choosing one.

## Evidence presets

A policy says **what use is allowed or prohibited**. A preset says **which technical evidence channels to use**. They are intentionally separate.

| Preset | Rights metadata | Hidden marker | Authentication |
|---|---:|---:|---:|
| `legal-notice` | Yes | No | No |
| `legal-notice-with-stego` | Yes | Best effort | No |
| `authenticated-provenance` | Yes | Best effort | HMAC (key required) |
| `maximal` | Yes | Best effort | HMAC (key required) |

For `authenticated-provenance` and `maximal`, provide a secret key with `--key` or `STEGOEGGO_KEY`. HMAC authentication establishes that the hidden payload was produced by someone holding that secret; it does **not** prove copyright ownership or authorship.

Without a MAC key, hidden-payload integrity uses non-cryptographic checks intended for detection and corruption checking, not adversarial authentication.

## What is written

Rights metadata is the primary signal. The byte-processing path builds one normalized rights notice and writes format-appropriate metadata. A specified `RightsPolicy` is represented with canonical PLUS `plus:DataMining` XMP metadata. Optional fields include copyright notice, creator, rights URL, usage terms, AI constraints, credit/licensor information, and relevant dates.

Existing unrelated metadata is preserved where the format-specific update path supports it, while StegoEggo-owned fields are replaced by default when an image is processed again. The library also exposes metadata conflict/update policies for callers that need stricter behavior.

Hidden markers are an optional secondary signal:

| Format | Read/write | Rights metadata | Hidden marker |
|---|---:|---:|---|
| PNG | Yes | Yes | Pixel-domain LSB, best effort |
| JPEG | Yes | Yes | DCT-domain embedding on supported JPEG structures; fallback signals may be used when full embedding is unsupported |
| WebP | Yes | Yes | Pixel-domain LSB for lossless WebP; lossy WebP stego is not supported |

If hidden-marker recoverability matters, PNG is the most predictable output format. Do not treat any hidden marker as guaranteed to survive arbitrary image transformations.

## Rust API

The canonical library interface is `ProtectionRequest` + `RightsPolicy`. For metadata that must remain in the encoded file, use the byte APIs such as `process_request_bytes`.

```rust
use stegoeggo::{
    process_request_bytes, ProtectionRequest, RightsNotice, RightsPolicy,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::fs::read("image.png")?;

    let notice = RightsNotice::new()
        .with_copyright_holder("Example Artist")
        .with_creator("Example Artist")
        .with_usage_terms("No AI/ML training.")
        .with_web_statement_of_rights("https://example.com/rights");

    let request = ProtectionRequest::metadata_only(
        notice,
        RightsPolicy::ProhibitedAiMlTraining,
    );

    let output = process_request_bytes(&input, &request)?;
    std::fs::write("image_protected.png", output)?;
    Ok(())
}
```

To request the hidden marker instead:

```rust
let request = ProtectionRequest::with_hidden_marker(
    notice,
    RightsPolicy::ProhibitedAiMlTraining,
);
```

The older `ProtectionContext`, `ProtectionLevel`, `EvidenceProfile`, `with_dmi()`, and related APIs remain functional compatibility surfaces but are deprecated for new code. See [DEPRECATIONS.md](DEPRECATIONS.md) and [docs/migration-v0.3.md](docs/migration-v0.3.md).

### Byte APIs versus `DynamicImage`

This distinction matters: file metadata lives in the encoded image container. APIs that accept and return `image::DynamicImage` operate on decoded pixels and cannot preserve or inject file-level metadata by themselves. Use `process_request_bytes` (canonical) or the legacy `process_image_bytes` path when the resulting file must contain rights metadata.

## Verification

`--verify` and the library verification APIs inspect both metadata and hidden evidence. Reports distinguish metadata-only notices, best-effort steganographic evidence, and HMAC-authenticated provenance when a matching key is supplied.

Verification should be interpreted as evidence about what is present in the file, not as a legal conclusion. Metadata can be copied or forged; an HMAC proves knowledge of a secret key, not ownership of the underlying work.

The CLI's compatibility `--verify` mode exits successfully after producing a report; automation that needs structured results should prefer `--json` and inspect the report fields rather than treating the process exit code as a protected/not-protected boolean.

## Feature flags

The library keeps optional functionality behind Cargo features:

| Feature | Purpose |
|---|---|
| `async` | Tokio-based async wrappers |
| `signatures` | Ed25519 signing support |
| `detached-manifest` | Detached signed-manifest support |
| `iscc` | Content identifier helpers |
| `parallel` | Rayon-based parallel processing |
| `conformance` | Conformance harness and manifest parsing |

No optional feature is enabled by default. The CLI enables the application features it needs internally; its `keygen`, `sign`, and `verify-manifest` subcommands are available only when `stegoeggo-cli` is built with its `signatures` feature.

The workspace also contains [`stegoeggo-stego`](stegoeggo-stego/), a lower-level, application-neutral carrier crate for callers that want generic LSB/JPEG steganography without StegoEggo's rights-policy layer.

Internally, the rights-aware hidden-marker adapter is organized by responsibility under `src/protected/steganography/`: marker construction, carrier embedding, extraction/search, verification, and legacy compatibility are separate modules behind the existing `SteganographyProtector` facade.

## Standards and compatibility

Current rights-policy output uses the PLUS License Data Format `plus:DataMining` property with the canonical controlled-vocabulary URI for the selected policy. The verifier also recognizes older bare PLUS values, legacy `Iptc4xmpExt:DMI-*` data, and legacy TDM reservation metadata so existing files can still be inspected.

The current hidden payload writer emits V3 payloads. V1 and V2 payloads remain readable for compatibility. Detached manifests use their current V1 format when that feature is enabled.

StegoEggo deliberately does not claim C2PA compatibility or robust forensic watermarking. Those are separate trust and provenance models.

## Safety and legal scope

Only assert copyright, licensing, or usage restrictions that you are entitled to assert. StegoEggo records a notice and optional technical evidence; it does not create rights you do not already have and is not legal advice.

For security-sensitive deployments, treat unauthenticated hidden markers as forgeable. Use HMAC-authenticated provenance when origin authentication is required, protect the key outside the image, and keep the original source material and independent provenance records.

## Project documentation

- [SUPPORT.md](SUPPORT.md) — MSRV, platforms, formats, and supported feature surface
- [STABILITY.md](STABILITY.md) — compatibility and stability policy
- [DEPRECATIONS.md](DEPRECATIONS.md) — deprecated APIs and replacements
- [SECURITY.md](SECURITY.md) — security policy and reporting
- [docs/legal_notice_model.md](docs/legal_notice_model.md) — rights-notice and evidence model
- [architecture/](architecture/) — implementation and protocol documentation

For repository development, the fast required check is:

```bash
./scripts/check.sh
```

It runs formatting, strict Clippy, a minimal-feature compile, and the all-features workspace test suite. Specialist conformance, fuzzing, MSRV, packaging, and external-tool checks are intentionally separate from the normal push CI path.

## License

MIT. See [LICENSE](LICENSE).
