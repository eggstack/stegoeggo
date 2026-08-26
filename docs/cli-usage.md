# CLI Usage

## Installation

Install from crates.io:

```bash
cargo install stegoeggo-cli
```

Or build from source:

```bash
git clone https://github.com/eggstack/stegoeggo.git
cd stegoeggo
cargo build --release --bin stegoeggo
./target/release/stegoeggo --help
```

## Protecting images

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

## Inspecting files

```bash
stegoeggo image_protected.png --verify
```

Machine-readable verification output is available with `--json`:

```bash
stegoeggo image_protected.png --verify --json
```

## Batch processing

A directory can be processed as a batch. `-j` controls worker count:

```bash
stegoeggo ./images -o ./protected \
  --rights-policy prohibited-ai-ml-training \
  --preset legal-notice \
  -j 4
```

Input format is detected from the image data. Unless `--format` is supplied, the CLI preserves the input format. With no explicit output path, protected files use a `_protected` suffix.

## CLI defaults

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

## Subcommands (feature: `signatures`)

| Command | Feature Gate | Description |
|---------|-------------|-------------|
| `stegoeggo keygen` | `signatures` | Generate an Ed25519 key pair |
| `stegoeggo sign --manifest <path> --key <path>` | `signatures` | Sign a detached manifest |
| `stegoeggo verify-manifest --manifest <path> --image <path>` | `signatures` | Verify a detached manifest against an image |

## Exit codes

| Code | Constant | Meaning |
|------|----------|---------|
| 0 | `EXIT_OK` | Success |
| 1 | `EXIT_ERROR` | General error (I/O, image decode/encode, etc.) |
| 2 | `EXIT_CONFIG` | Malformed manifest, config error, or input validation failure |
| 3 | `EXIT_INTEGRITY` | Digest mismatch, binding failure, or signature/integrity failure |
| 4 | — | `verify-manifest`: cryptographically verified but untrusted |
| 5 | `EXIT_INTERNAL` | Internal or unexpected error |

The `--verify` flag always exits 0; use output text to determine protection state, not the process exit code.
