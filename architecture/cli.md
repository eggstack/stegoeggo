# CLI Tool

**Source:** `stegoeggo-cli/src/` — `args.rs` (clap topology), `main.rs`
(routing), `request.rs` (normalization), `protect.rs` (execution), `verify.rs`
(inspection and assertion), `update.rs` (version resolution and self-update),
`output.rs` (formatting and exit mapping), `keys.rs`, and `manifest.rs`
(feature-gated detached-manifest commands).

The CLI has a command-oriented surface while retaining the flag-first 0.x
syntax as an implicit `protect` alias. All image protection requests pass
through one `ProtectionRequest` builder.

## Command topology

```text
stegoeggo protect <INPUT>...
stegoeggo inspect <IMAGE>
stegoeggo verify <IMAGE>
stegoeggo version
stegoeggo update
```

The `keygen`, `sign`, and `verify-manifest` commands remain flat and are
compiled only with the `signatures` feature. `version` prints the compile-time
package version as its stable first line and has no network or configuration
inputs. `update` uses crates.io `stegoeggo-cli` stable versions as its authority
and the matching GitHub Release as its binary source.

The update flow is:

```text
crates.io stable version
        ↓
current-version comparison ── current is newest → exit 0
        ↓
replaceability preflight and host-target mapping
        ↓
version-tagged binary + SHA-256 sidecar download
        ↓
checksum → executable permissions → candidate version/identity
        ↓
self-replace at the running executable path
```

Only an unsupported target or HTTP 404 for the exact binary asset may invoke
`cargo install stegoeggo-cli --locked --version =X.Y.Z`. Curl, candidate, and
replacement subprocesses are bounded and use argument arrays; no shell command
strings or implicit privilege escalation are used. A staged failure leaves the
current executable untouched.

### Compatibility routing

`main::uses_command_parser()` recognizes a command token after leading CLI
options. A recognized command is parsed by `RootArgs`; all other invocations
are parsed by the legacy-compatible `Args` wrapper and routed to `run_protect`.
This preserves invocations such as:

```bash
stegoeggo image.png --level standard --profile legal-notice-stego
stegoeggo image.png --verify
```

The exact tokens `protect`, `inspect`, `verify`, `version`, and `update` are
therefore command names when used as the first positional token. A file or
directory with one of those names must be written as an unambiguous path such
as `./verify` or `-- ./verify`. The router skips values for known value-taking
options, so a value such as `--key version` is not mistaken for a command.

## Protection arguments

`ProtectArgs` is shared by the explicit `protect` command and the implicit
root compatibility parser. Its help headings group the options into:

- Policy and evidence: `--rights-policy`, `--preset`, `--hidden-marker`,
  `--authentication`, and `--key`.
- Rights metadata: legal-notice fields and `--legal-claims`.
- Image output: `--output`, `--format`, JPEG quality, progressive encoding, and
  stego redundancy.
- Execution: seed, intensity, batch jobs, JSON, strict, verbose, and dry run.
- Compatibility (legacy): `--level`, `--profile`, `--dmi`, metadata controls,
  and AI/TDM shorthands.

The compatibility `--verify` option is hidden in generated protection help but
remains parseable. The top-level help is generated from `RootArgs`, so it lists
commands without dumping the legacy protection surface.

## Request normalization

All explicit and implicit protection modes (single-file, batch, dry-run, and
JSON) call:

```text
request::build_protection_request_with_explicit_options()
    -> ProtectionRequest
    -> process_request_bytes_with_warnings/report()
```

The builder translates legacy flags into the canonical request model; it does
not maintain a second processing pipeline.

Modern fields are `--rights-policy`, `--preset`, `--hidden-marker`, and
`--authentication`. Legacy policy sources are `--dmi`, `--no-ai-training`,
`--no-genai-training`, and `--tdm-reserved`. Legacy channel sources are
`--level` and `--profile`.

Explicit modern fields take precedence over translated legacy defaults.
Contradictory explicit combinations return exit code 2. In particular, the
builder rejects:

- `--preset` with explicit `--level` or `--profile`;
- `--preset` with `--hidden-marker` or `--authentication`;
- explicit channel flags with explicit legacy channel flags;
- contradictory policy sources;
- `--metadata false` with legal metadata or metadata-injecting channels;
- HMAC without a key or with a disabled hidden marker.

Legacy defaults remain unchanged: `standard` plus `legal-notice` resolves to
rights metadata, a best-effort hidden marker, and
`ProhibitedAiMlTraining`. `authenticated-provenance` and `maximal` currently
expand to the same metadata + best-effort marker + HMAC channel set; both names
remain available through 0.x.

## Inspection and verification

`inspect` reads the image and prints the rights fields, detected policy/evidence
channels, hidden-marker status, authentication state, payload details, and
evidence strength. It normally exits 0 whenever the image can be inspected,
including an unprotected image. `--json` retains the compatibility verification
object with `schema_version: 1`.

`verify` uses the same `verify_legal_notice()` canonical-facts projection as
`inspect`, but is assertion-oriented. It exits 3 when a marker is present but
integrity/authentication verification fails, or when no protection evidence is
found. A valid metadata-only notice satisfies the assertion even without a
hidden marker. Runtime/I/O errors use exit 1, invalid configuration uses exit 2,
and unexpected failures use exit 5. JSON output is emitted before the non-zero
verification result, with `status: "failed"`.

The compatibility root `--verify` path keeps its historical always-zero exit
behavior and may read an explicit output file supplied with `--output`.

## Input and output handling

- A single input writes `{stem}_protected.{ext}` or the path supplied by
  `--output`.
- Multiple inputs or a directory use batch processing and a flat output
  directory; duplicate stems receive `_protected_N` suffixes.
- Input format is detected from magic bytes. `--format` overrides it; otherwise
  the input format is preserved.
- `--dry-run` resolves and prints the protection plan without writing files.
- `--json` on `protect` reports the existing execution schema. `--json` on
  `inspect`/`verify` reports the compatibility verification schema.

## Exit codes

| Code | Constant | Meaning |
|---|---|---|
| 0 | `EXIT_OK` | Operation succeeded; inspection can report no protection |
| 1 | `EXIT_ERROR` | I/O, image decode/encode, or general runtime error |
| 2 | `EXIT_CONFIG` | Invalid invocation or configuration |
| 3 | `EXIT_INTEGRITY` | `verify` assertion or payload/authentication failure |
| 4 | — | `verify-manifest`: verified but untrusted |
| 5 | `EXIT_INTERNAL` | Unexpected/internal failure |

## Modules and dependencies

| File | Responsibility |
|---|---|
| `args.rs` | Root/command clap structures and value enums |
| `request.rs` | Canonical request construction, precedence validation, legal metadata, warnings |
| `protect.rs` | Single/batch processing, output paths, atomic writes |
| `verify.rs` | Inspection and verification rendering, JSON compatibility output |
| `output.rs` | Human/JSON types and exit-code mapping |
| `keys.rs` | Hex, `@file`, stdin, and `STEGOEGGO_KEY` resolution |
| `manifest.rs` | `signatures`-gated detached-manifest operations |
| `update.rs` | Stable version lookup, release asset verification, Cargo fallback, and self-replacement |
| `main.rs` | Parser selection, command dispatch, and orchestration |

Production dependencies are clap 4, the `stegoeggo` library, rayon for
error-tolerant CLI batches, hex, serde/serde_json, sha2, self-replace, and
tempfile. The CLI does
not enable the library's `parallel`, `iscc`, or `conformance` features. The
package's default feature is `signatures`, which adds the detached-manifest
commands to Cargo-installed and prebuilt binaries. Release asset names,
checksum handling, and installer fallback rules are documented in
[`docs/installation.md`](../docs/installation.md) and
[`RELEASING.md`](../RELEASING.md); they are not part of the image-processing
command router.
