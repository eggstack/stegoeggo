# stegoeggo

[![CI](https://github.com/eggstack/stegoeggo/actions/workflows/ci.yml/badge.svg)](https://github.com/eggstack/stegoeggo/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/stegoeggo)](https://crates.io/crates/stegoeggo)
[![Crates.io downloads](https://img.shields.io/crates/d/stegoeggo)](https://crates.io/crates/stegoeggo)
[![Documentation](https://docs.rs/stegoeggo/badge.svg)](https://docs.rs/stegoeggo)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.87-blue.svg)](https://blog.rust-lang.org/)

Embed machine-readable rights-reservation metadata and AI-training restriction notices in images, with optional best-effort steganographic markers for redundant evidence.

`stegoeggo` is primarily a **rights-notice metadata tool**. It writes explicit rights policy and copyright information into PNG, JPEG, and WebP files. A hidden marker can also be added as a second, best-effort evidence channel.

It is not DRM, a forensic watermark, a data-poisoning system, or proof that a particular model trained on an image. Metadata can be stripped, and hidden markers can be damaged or removed by transformations such as screenshots, cropping, resizing, or re-encoding.

## Installation

```bash
cargo install stegoeggo-cli
```

Or build from source:

```bash
git clone https://github.com/eggstack/stegoeggo.git
cd stegoeggo
cargo build --release --bin stegoeggo
```

For the Rust library:

```toml
[dependencies]
stegoeggo = "0.3"
```

The minimum supported Rust version is **1.87**. See [SUPPORT.md](SUPPORT.md) for the maintained platform and feature matrix.

## Quick start

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

For batch processing, subcommands, and all CLI flags, see [docs/cli-usage.md](docs/cli-usage.md).

## Rights policies

| CLI value | Meaning |
|---|---|
| `unspecified` | Do not emit a `plus:DataMining` policy value |
| `allowed` | Data mining allowed |
| `prohibited-ai-ml-training` | AI/ML training prohibited |
| `prohibited-generative-ai-training` | Generative-AI training prohibited |
| `prohibited-except-search-indexing` | Data mining prohibited except search-engine indexing |
| `prohibited-all-data-mining` | All data mining prohibited |
| `prohibited-see-constraints` | Prohibited; consult the supplied constraints |

## Evidence presets

A policy says **what use is allowed or prohibited**. A preset says **which technical evidence channels to use**.

| Preset | Rights metadata | Hidden marker | Authentication |
|---|---:|---:|---:|
| `legal-notice` | Yes | No | No |
| `legal-notice-with-stego` | Yes | Best effort | No |
| `authenticated-provenance` | Yes | Best effort | HMAC (key required) |
| `maximal` | Yes | Best effort | HMAC (key required) |

## Rust API

```rust
use stegoeggo::{
    process_request_bytes, ProtectionRequest, RightsNotice, RightsPolicy,
};

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
```

For byte APIs vs `DynamicImage`, verification, and the deprecated compatibility surface, see [docs/rust-api.md](docs/rust-api.md).

## Feature flags

| Feature | Purpose |
|---|---|
| `async` | Tokio-based async wrappers |
| `signatures` | Ed25519 signing support |
| `detached-manifest` | Detached signed-manifest support |
| `iscc` | Content identifier helpers |
| `parallel` | Rayon-based parallel processing |
| `conformance` | Conformance harness and manifest parsing |

No optional feature is enabled by default. See [SUPPORT.md](SUPPORT.md) for the full feature matrix.

## Documentation

| Document | Description |
|---|---|
| [docs/cli-usage.md](docs/cli-usage.md) | CLI flags, batch processing, exit codes |
| [docs/rust-api.md](docs/rust-api.md) | Rust API examples, byte vs DynamicImage |
| [docs/formats.md](docs/formats.md) | Format support, steganography details, transformation effects |
| [docs/carrier-crate.md](docs/carrier-crate.md) | `stegoeggo-stego` generic carrier crate |
| [docs/legal_notice_model.md](docs/legal_notice_model.md) | Rights-notice and evidence model |
| [docs/migration-v0.3.md](docs/migration-v0.3.md) | Migration guide from v0.2.x |
| [SUPPORT.md](SUPPORT.md) | MSRV, platforms, formats, feature surface |
| [STABILITY.md](STABILITY.md) | Stability tiers and retention promises |
| [DEPRECATIONS.md](DEPRECATIONS.md) | Deprecated APIs and replacements |
| [SECURITY.md](SECURITY.md) | Security policy and reporting |
| [architecture/](https://github.com/eggstack/stegoeggo/tree/main/architecture) | Implementation and protocol documentation |

## Safety and legal scope

Only assert copyright, licensing, or usage restrictions that you are entitled to assert. StegoEggo records a notice and optional technical evidence; it does not create rights you do not already have and is not legal advice.

For security-sensitive deployments, treat unauthenticated hidden markers as forgeable. Use HMAC-authenticated provenance when origin authentication is required, protect the key outside the image, and keep the original source material and independent provenance records.

## License

MIT. See [LICENSE](LICENSE).
