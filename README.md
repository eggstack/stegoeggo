# stegoeggo

[![CI](https://github.com/eggstack/stegoeggo/actions/workflows/ci.yml/badge.svg)](https://github.com/eggstack/stegoeggo/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/stegoeggo)](https://crates.io/crates/stegoeggo)
[![Documentation](https://docs.rs/stegoeggo/badge.svg)](https://docs.rs/stegoeggo)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.89-blue.svg)](https://blog.rust-lang.org/)

Write machine-readable rights notices and AI-training policies into PNG, JPEG,
and WebP images. An optional hidden marker provides a second, best-effort
evidence channel.

StegoEggo records notices. It is not DRM, a forensic watermark, or proof that a
model used an image. Metadata can be changed or removed, and image processing
can damage hidden markers.

## Quick start

Install a prebuilt CLI on a supported Unix target:

~~~bash
curl -fsSL https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh | bash
~~~

On Windows PowerShell:

~~~powershell
irm https://github.com/eggstack/stegoeggo/releases/latest/download/install.ps1 | iex
~~~

Write rights metadata and an AI/ML training prohibition:

~~~bash
stegoeggo protect image.png -o protected.png \
  --rights-policy prohibited-ai-ml-training \
  --preset legal-notice \
  --copyright-notice "© 2026 Example Artist. All rights reserved." \
  --creator "Example Artist" \
  --rights-url "https://example.com/rights"
~~~

Read the notice and check the protection evidence:

~~~bash
stegoeggo inspect protected.png
stegoeggo verify protected.png
~~~

`inspect` is read-only and exits successfully for a readable image even when it
has no protection evidence. `verify` is intended for scripts and exits 3 when
evidence is missing or invalid.

## Policies and evidence

A **rights policy** states the policy recorded in metadata. A **preset** selects
the technical evidence channels independently.

| `--rights-policy` | Meaning |
|---|---|
| `unspecified` | Do not write a PLUS data-mining policy value |
| `allowed` | Data mining allowed |
| `prohibited-ai-ml-training` | AI/ML training prohibited |
| `prohibited-generative-ai-training` | Generative-AI training prohibited |
| `prohibited-except-search-indexing` | Prohibited except search-engine indexing |
| `prohibited-all-data-mining` | All data mining prohibited |
| `prohibited-see-constraints` | Prohibited; consult the supplied constraints |

With the default `standard` level and no explicit policy, the CLI uses
`prohibited-ai-ml-training`. To write no PLUS policy value, set
`--rights-policy unspecified`. Choosing a preset does not change the policy.

| `--preset` | Rights metadata | Hidden marker | Authentication |
|---|---:|---:|---:|
| `legal-notice` | Yes | No | No |
| `legal-notice-with-stego` | Yes | Best effort | No |
| `authenticated-provenance`, `maximal` | Yes | Best effort | HMAC, key required |

HMAC authenticates a marker for holders of the shared secret. It does not prove
author identity or copyright ownership. Keep the key secret and outside the
image. See the [CLI guide](docs/cli-usage.md) for key options.

## Rust library

~~~toml
[dependencies]
stegoeggo = "0.4"
~~~

Use the byte API when metadata must be written to the encoded image:

~~~rust
use stegoeggo::{process_request_bytes, ProtectionRequest, RightsNotice, RightsPolicy};

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
    std::fs::write("protected.png", output)?;
    Ok(())
}
~~~

The `DynamicImage`-based `process_image` API works on pixels and does not preserve
file-level metadata. For generic payload embedding without the rights policy
layer, use the separate
[`stegoeggo-stego` crate](https://crates.io/crates/stegoeggo-stego).

## Documentation

| Guide | Covers |
|---|---|
| [CLI usage](docs/cli-usage.md) | Common commands, options, and exit codes |
| [Installation](docs/installation.md) | Install, update, supported targets, and Cargo fallback |
| [Rust API](docs/rust-api.md) | Byte APIs, verification, async, and batch processing |
| [Format support](docs/formats.md) | Metadata and hidden-marker behavior by image format |
| [Rights notice model](docs/legal_notice_model.md) | What notices and verification evidence mean |
| [Carrier crate](docs/carrier-crate.md) | Generic LSB and JPEG DCT carrier API |
| [Migration from v0.2.x](docs/migration-v0.3.md) | Historical changes introduced in v0.3.0 |
| [Support matrix](SUPPORT.md) | Supported Rust, platforms, formats, and bindings |
| [Stability](STABILITY.md) | Public API stability guarantees |
| [Deprecations](DEPRECATIONS.md) | Deprecated APIs and compatibility syntax |
| [Security policy](SECURITY.md) | Vulnerability reporting and security limits |

## Important limitations

Only record claims you are entitled to make. StegoEggo does not create rights,
enforce restrictions, or provide legal advice. Metadata may be copied, edited,
or stripped. Hidden markers are best effort and may not survive re-encoding,
resizing, or cropping. Keep original files and independent provenance records
when those matter.

## License

MIT. See [LICENSE](LICENSE).
