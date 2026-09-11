# CLI Usage

## Installation

Install the prebuilt CLI on supported platforms without Rust:

```bash
curl -fsSL https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh | bash
```

Windows PowerShell:

```powershell
irm https://github.com/eggstack/stegoeggo/releases/latest/download/install.ps1 | iex
```

For pinned releases, pass `--version X.Y.Z` to the Unix installer. See
[the installation guide](installation.md) for target coverage and checksum
details.

Cargo remains a supported fallback:

```bash
cargo install stegoeggo-cli --locked
```

Or build from source:

```bash
git clone https://github.com/eggstack/stegoeggo.git
cd stegoeggo
cargo build --release --bin stegoeggo
./target/release/stegoeggo --help
```

## Commands

The command-oriented interface is canonical:

```text
stegoeggo protect <INPUT>...
stegoeggo inspect <IMAGE>
stegoeggo verify <IMAGE>
stegoeggo version
stegoeggo update
```

Feature-gated signing commands remain flat:

```text
stegoeggo keygen
stegoeggo sign --manifest <path> --key <path>
stegoeggo verify-manifest --manifest <path> --image <path>
```

They require the `signatures` feature.

The published CLI package enables `signatures` by default, so these commands
are also present in Cargo-installed and prebuilt binaries.

## Version and updates

`version` prints exactly `stegoeggo X.Y.Z` on its first line and does not access
the network or configuration. `update` resolves the latest stable CLI version
from crates.io and updates from the matching verified GitHub Release asset;
see [the installation guide](installation.md) for its fallback and permission
rules. Update progress is written to stderr and its final result to stdout.

## Protecting images

Write a metadata-only AI/ML training prohibition:

```bash
stegoeggo protect image.png -o image_protected.png \
  --rights-policy prohibited-ai-ml-training \
  --preset legal-notice \
  --copyright-notice "© 2026 Example Artist. All rights reserved." \
  --creator "Example Artist" \
  --rights-url "https://example.com/rights" \
  --usage-terms "No AI/ML training."
```

Add a best-effort hidden marker as a redundant channel:

```bash
stegoeggo protect image.png -o image_protected.png \
  --rights-policy prohibited-ai-ml-training \
  --preset legal-notice-with-stego
```

Input format is detected from the image data. Unless `--format` is supplied,
the CLI preserves the input format. With no explicit output path, protected
files use a `_protected` suffix. A directory or multiple inputs enables flat
batch processing; `-j` controls the worker count.

## Inspecting and verifying files

`inspect` is read-only and exits 0 whenever the image can be parsed, including
an unprotected image:

```bash
stegoeggo inspect image_protected.png
stegoeggo inspect image_protected.png --json
```

`verify` is for scripts or release checks. It prints the same report but exits
3 when a marker fails integrity/authentication verification or when no
protection evidence is found. A valid metadata-only notice is sufficient:

```bash
stegoeggo verify image_protected.png --key @key.bin
stegoeggo verify image_protected.png --json
```

The compatibility root form remains accepted during 0.x:

```bash
stegoeggo image_protected.png --verify
```

It keeps the historical always-zero exit behavior. Use `inspect` for the
interactive replacement and `verify` when process status matters.

## Canonical policy and evidence options

`--rights-policy` maps directly to the library's `RightsPolicy` enum and, when
specified, to the corresponding PLUS `plus:DataMining` XMP value.

| CLI value | Meaning |
|---|---|
| `unspecified` | Do not emit a `plus:DataMining` policy value |
| `allowed` | Data mining allowed |
| `prohibited-ai-ml-training` | AI/ML training prohibited |
| `prohibited-generative-ai-training` | Generative-AI training prohibited |
| `prohibited-except-search-indexing` | Data mining prohibited except search-engine indexing |
| `prohibited-all-data-mining` | All data mining prohibited |
| `prohibited-see-constraints` | Prohibited; consult the supplied constraints |

`--preset` selects technical evidence channels independently of policy:

| Preset | Rights metadata | Hidden marker | Authentication |
|---|---:|---:|---:|
| `legal-notice` | Yes | No | No |
| `legal-notice-with-stego` | Yes | Best effort | No |
| `authenticated-provenance` | Yes | Best effort | HMAC (key required) |
| `maximal` | Yes | Best effort | HMAC (key required) |

`authenticated-provenance` and `maximal` currently resolve to the same
`ProtectionRequest` channel set. Both remain supported in 0.x; consolidation is
a v1 candidate. HMAC authentication shows that the marker was produced with
the supplied secret. It does not prove copyright ownership or authorship.

Canonical channel flags are `--hidden-marker disabled|best-effort` and
`--authentication none|hmac`. HMAC requires `--key` (hex, `@file`, stdin `-`,
or `STEGOEGGO_KEY`).

## Rights metadata fields

The common rights fields are supplied directly to `protect`:

| Flag | Purpose |
|---|---|
| `--copyright-notice` | Copyright notice text |
| `--creator` | Creator or author name |
| `--contact` | Rights contact email or URL |
| `--rights-url` | URL to full terms or license text |
| `--usage-terms` | Short usage-terms summary |
| `--ai-constraints` | AI-specific constraints |
| `--credit-line` | Required attribution line |
| `--copyright-owner` | Copyright owner name |
| `--licensor-name`, `--licensor-email`, `--licensor-url` | Structured licensor details |
| `--content-created-at` | ISO 8601 content creation date |

Only provide claims you are entitled to assert. `--legal-claims` is a legacy
compatibility flag; supplying canonical rights fields is the recommended path.

## Compatibility syntax and precedence

The old root invocation remains a compatibility alias for `protect`. The
legacy `--level`, `--profile`, `--dmi`, `--metadata`, `--legal-claims`, and
AI/TDM shorthand options are still accepted but are not the recommended
interface. They are translation-only inputs to the same canonical builder.

All protection modes share one `ProtectionRequest` builder:

1. Modern fields win when they explicitly specify the same field.
2. Legacy flags translate only when the modern field is absent.
3. Contradictory explicit combinations fail with exit code 2.
4. Defaults are applied once, after explicitness is known.

Examples of rejected combinations include `--preset` with explicit
`--level`/`--profile`, `--preset` with explicit channel flags, contradictory
policy sources, `--metadata false` with legal fields or metadata-injecting
channels, and HMAC without a key.

The exact command names `protect`, `inspect`, `verify`, `version`, and `update`
take precedence over a same-named first positional token. Use `./verify` or
`-- ./verify` for an image path with an ambiguous name.

## Exit codes

| Code | Meaning |
|---:|---|
| 0 | Success; `inspect` may report no protection |
| 1 | I/O, image decode/encode, or general runtime error |
| 2 | Invalid invocation or configuration |
| 3 | `verify` assertion or payload/authentication failure |
| 4 | `verify-manifest`: cryptographically verified but untrusted |
| 5 | Unexpected/internal failure |

The `--verify` compatibility flag always exits 0; read its output to determine
the reported protection state.
