# CLI Usage

On supported Unix targets, install the prebuilt CLI with:

~~~bash
curl -fsSL https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh | bash
~~~

Windows users and users who need Cargo installation or platform details can
see the [installation guide](installation.md).

## Commands

~~~text
stegoeggo protect <INPUT>...
stegoeggo inspect <IMAGE>
stegoeggo verify <IMAGE>
stegoeggo version
stegoeggo update
~~~

The published CLI also includes these signing commands:

~~~text
stegoeggo keygen
stegoeggo sign --manifest <path> --key <path>
stegoeggo verify-manifest --manifest <path> --image <path>
~~~

They are enabled by default in the published CLI package and prebuilt binaries.
`keygen` writes `key_private.pem` and `key_public.pem` in the current directory
by default. Use `--output-dir` to choose another location. It refuses to
overwrite existing key files, so store the private key securely.

## Protect an image

Write rights metadata without a hidden marker:

~~~bash
stegoeggo protect image.png -o protected.png \
  --rights-policy prohibited-ai-ml-training \
  --preset legal-notice \
  --copyright-notice "© 2026 Example Artist. All rights reserved." \
  --creator "Example Artist" \
  --rights-url "https://example.com/rights" \
  --usage-terms "No AI/ML training."
~~~

Add an optional best-effort hidden marker:

~~~bash
stegoeggo protect image.png -o protected.png \
  --rights-policy prohibited-ai-ml-training \
  --preset legal-notice-with-stego
~~~

The input format is detected from the image. Unless `--format` is set, the CLI
keeps that format. Without an explicit output path, it adds `_protected` to the
filename.

## Choose policy and evidence

`--rights-policy` selects the rights policy recorded in metadata:

| Value | Meaning |
|---|---|
| `unspecified` | Do not write a PLUS data-mining policy value |
| `allowed` | Data mining allowed |
| `prohibited-ai-ml-training` | AI/ML training prohibited |
| `prohibited-generative-ai-training` | Generative-AI training prohibited |
| `prohibited-except-search-indexing` | Prohibited except search-engine indexing |
| `prohibited-all-data-mining` | All data mining prohibited |
| `prohibited-see-constraints` | Prohibited; consult the supplied constraints |

With the default `standard` level and no explicit policy, the CLI uses
`prohibited-ai-ml-training`. Choosing a preset does not change that policy. To
write no PLUS policy value, set `--rights-policy unspecified`.

`--preset` chooses the evidence channels independently of policy:

| Preset | Rights metadata | Hidden marker | Authentication |
|---|---:|---:|---:|
| `legal-notice` | Yes | No | No |
| `legal-notice-with-stego` | Yes | Best effort | No |
| `authenticated-provenance`, `maximal` | Yes | Best effort | HMAC (key required) |

For direct channel control, use `--hidden-marker disabled|best-effort` and
`--authentication none|hmac` instead of `--preset`. Do not combine a preset
with explicit channel flags. HMAC requires a non-disabled hidden marker and a
key. HMAC verifies that a marker matches a shared secret; it does not establish
a public identity, ownership, or authorship.

The `--key` option accepts a hex string, `@path` to a file containing hex,
stdin with `-`, or the `STEGOEGGO_KEY` environment variable. Protect key files
and do not store the secret in the image or source control.

## Rights metadata fields

Use only claims you are entitled to make.

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

## Batch processing and output

Pass multiple files or a directory to process a batch. Batch output is flat, and
`-j` controls the worker count.

`--output` is treated as a file when it has an extension or already exists as
a file. Otherwise it is treated as an output directory. A batch writes one
output per input and requires a directory output path, so a file-valued output
path is rejected for batch processing.

Use `--json` for machine-readable output. For `protect`, a batch produces one
JSON document with a `files` array. `--dry-run` prints the plan without writing
protected output.

## Inspect and verify

`inspect` reads the image and reports any protection evidence. It exits `0`
for a readable unprotected image:

~~~bash
stegoeggo inspect image.png
stegoeggo inspect image.png --json
~~~

`verify` is intended for scripts. It exits `3` when protection evidence is
missing or invalid. A valid metadata-only notice is sufficient evidence:

~~~bash
stegoeggo verify protected.png
stegoeggo verify protected.png --key @mac-key.txt
stegoeggo verify protected.png --json
~~~

Use `--key` when verifying an HMAC marker. Without the matching key, the
report cannot authenticate that marker.

## Compatibility syntax

During 0.x, the old positional form `stegoeggo <input>...` remains an alias for
`protect`. The legacy `--verify` flag is verify-only and always exits `0`;
use the `verify` command when process status matters. Older flags such as
`--level`, `--profile`, and `--dmi` remain for compatibility, but new
scripts should use `--rights-policy`, `--preset`, and the channel options.
See [DEPRECATIONS.md](../DEPRECATIONS.md) for the full compatibility inventory.

## Exit codes

| Code | Meaning |
|---:|---|
| 0 | Success; `inspect` may report no protection |
| 1 | I/O, image decode/encode, or general runtime error |
| 2 | Invalid invocation or configuration |
| 3 | `verify` evidence or integrity assertion failed |
| 4 | `verify-manifest`: valid signature, but signer is untrusted |
| 5 | Unexpected/internal failure |

The legacy `--verify` flag always exits `0`; read its output to determine
the reported protection state. Invalid combinations, such as
`--verify --output`, fail with exit code `2` before verification.
