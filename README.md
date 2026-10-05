# stegoeggo

[![CI](https://github.com/eggstack/stegoeggo/actions/workflows/ci.yml/badge.svg)](https://github.com/eggstack/stegoeggo/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/stegoeggo)](https://crates.io/crates/stegoeggo)
[![Documentation](https://docs.rs/stegoeggo/badge.svg)](https://docs.rs/stegoeggo)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.89-blue.svg)](https://blog.rust-lang.org/)

Embed machine-readable rights-reservation metadata and AI-training restriction
notices in PNG, JPEG, and WebP images, with an optional best-effort
steganographic marker as a second evidence channel.

`stegoeggo` is primarily a **rights-notice metadata tool**. It writes explicit
rights policy and copyright information into the file. It is not DRM, a forensic
watermark, a data-poisoning system, or proof that a particular model trained on
an image — metadata can be stripped, and hidden markers can be damaged by
transformations such as screenshots, cropping, resizing, or re-encoding.

## Quick start

Install a prebuilt CLI (Unix):

```bash
curl -fsSL https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh | bash
```

Write an AI/ML training prohibition plus copyright metadata:

```bash
stegoeggo protect image.png -o protected.png \
  --rights-policy prohibited-ai-ml-training \
  --preset legal-notice \
  --copyright-notice "© 2026 Example Artist. All rights reserved." \
  --creator "Example Artist" \
  --rights-url "https://example.com/rights"
```

Read it back, then assert it is intact:

```bash
stegoeggo inspect protected.png
stegoeggo verify protected.png
```

`inspect` is read-only and exits `0` even for an unprotected file. `verify`
exits `3` when protection evidence is missing or invalid, which makes it usable
in scripts.

Add the best-effort hidden marker, or HMAC-authenticated provenance, by
switching `--preset`:

```bash
# metadata + best-effort hidden marker
stegoeggo protect image.png -o protected.png \
  --rights-policy prohibited-ai-ml-training \
  --preset legal-notice-with-stego

# metadata + hidden marker + HMAC authentication (key required)
stegoeggo protect image.png -o protected.png \
  --rights-policy prohibited-ai-ml-training \
  --preset authenticated-provenance \
  --key 00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff
```

A **policy** says what use is allowed or prohibited. A **preset** says which
technical evidence channels to use.

| `--rights-policy` | Meaning |
|---|---|
| `unspecified` | Emit no `plus:DataMining` policy value |
| `allowed` | Data mining allowed |
| `prohibited-ai-ml-training` | AI/ML training prohibited |
| `prohibited-generative-ai-training` | Generative-AI training prohibited |
| `prohibited-except-search-indexing` | Prohibited except search-engine indexing |
| `prohibited-all-data-mining` | All data mining prohibited |
| `prohibited-see-constraints` | Prohibited; consult the supplied constraints |

| `--preset` | Rights metadata | Hidden marker | Authentication |
|---|---:|---:|---:|
| `legal-notice` | Yes | No | No |
| `legal-notice-with-stego` | Yes | Best effort | No |
| `authenticated-provenance` | Yes | Best effort | HMAC (key required) |
| `maximal` | Yes | Best effort | HMAC (key required) |

## Rust library

```toml
[dependencies]
stegoeggo = "0.4"
```

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

Use the byte APIs above when metadata must survive. The `DynamicImage`-based
`process_image` only embeds a hidden marker — PNG `tEXt`, JPEG `COM`/XMP, and
WebP XMP do not survive that path.

For arbitrary-payload steganography without the rights layer, depend on
[`stegoeggo-stego`](https://crates.io/crates/stegoeggo-stego) directly.

## Documentation

| Document | Description |
|---|---|
| [docs/cli-usage.md](docs/cli-usage.md) | Every CLI command, flag, batch mode, and exit code |
| [docs/installation.md](docs/installation.md) | Installers, updates, targets, Cargo and source installs |
| [docs/rust-api.md](docs/rust-api.md) | Rust API: byte vs `DynamicImage`, verification, compatibility |
| [docs/formats.md](docs/formats.md) | Per-format support and what survives transformation |
| [docs/carrier-crate.md](docs/carrier-crate.md) | `stegoeggo-stego` generic carrier crate |
| [docs/legal_notice_model.md](docs/legal_notice_model.md) | Rights-notice and evidence model |
| [docs/migration-v0.3.md](docs/migration-v0.3.md) | Migration guide from v0.2.x |
| [SUPPORT.md](SUPPORT.md) | MSRV, platforms, formats, feature matrix |
| [STABILITY.md](STABILITY.md) | Stability tiers and retention promises |
| [DEPRECATIONS.md](DEPRECATIONS.md) | Deprecated APIs and replacements |
| [SECURITY.md](SECURITY.md) | Security policy and reporting |
| [architecture/](architecture/overview.md) | Implementation and protocol documentation |

## Safety and legal scope

Only assert copyright, licensing, or usage restrictions that you are entitled to
assert. StegoEggo records a notice and optional technical evidence; it does not
create rights you do not already have and is not legal advice.

For security-sensitive deployments, treat unauthenticated hidden markers as
forgeable. Use HMAC-authenticated provenance when origin authentication is
required, keep the key outside the image, and retain the original source
material and independent provenance records.

## License

MIT. See [LICENSE](LICENSE).
