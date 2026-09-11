# CLI Tool

**Source:** `stegoeggo-cli/src/` — `main.rs` (orchestration) plus private modules `args.rs`, `request.rs`, `protect.rs`, `verify.rs`, `output.rs`, `keys.rs`, `manifest.rs`

Command-line interface for `stegoeggo`. Built with `clap` 4 (derive). Routes all protection through one canonical `ProtectionRequest` builder.

## Binary

```bash
stegoeggo [OPTIONS] <INPUT>...
```

## Options

| Flag | Long | Description | Default |
|------|------|-------------|---------|
| `-o` | `--output` | Output directory (for batch) or output file (for single) | current directory |
| | `--verify` | Verify protection signature | false |
| `-l` | `--level` | Protection level | standard |
| `-p` | `--profile` | Evidence profile | legal-notice |
| `-i` | `--intensity` | Float 0.0–1.0 | 0.5 |
| `-s` | `--seed` | Seed for reproducibility | random |
| `-f` | `--format` | Output format (png/jpg/webp) — preserves input format, falls back to PNG | None (preserve input) |
| | `--stego-redundancy` | 1–10 | 2 |
| | `--jpeg-quality` | 1–100 | 90 |
| | `--progressive` | Progressive JPEG | false |
| `-v` | `--verbose` | Verbose output | false |
| `-d` | `--dmi` | DMI metadata value (legacy syntax) | auto |
| | `--metadata` | Inject metadata (None = use level default) | None |
| | `--legal-claims` | Inject legal claims | false |
| | `--copyright-notice` | Copyright notice text (alias: `--copyright-holder`) | none |
| | `--creator` | Creator/author name | none |
| | `--contact` | Contact email or URL | none |
| | `--rights-url` | URL to full usage terms | none |
| | `--usage-terms` | Brief usage terms summary | none |
| | `--credit-line` | Credit line text (e.g., 'Photo by Jane Doe / Acme Corp') | none |
| | `--copyright-owner` | Copyright owner name (distinct from copyright holder notice text) | none |
| | `--licensor-name` | Licensor name for PLUS structured rights | none |
| | `--licensor-email` | Licensor email for PLUS structured rights | none |
| | `--licensor-url` | Licensor URL for PLUS structured rights | none |
| | `--content-created-at` | Content creation date (ISO 8601) | none |
| | `--ai-constraints` | AI-specific constraints text | none |
| | `--no-ai-training` | Prohibit AI/ML training (DMI shorthand) | false |
| | `--no-genai-training` | Prohibit generative AI training (DMI shorthand) | false |
| | `--tdm-reserved` | Reserve TDM rights (DMI shorthand) — deprecated; sets ProhibitedSeeConstraints | false |
| | `--key` | Hex cryptographic key (hex, `@file`, `-` stdin, or `STEGOEGGO_KEY`) | none |
| `-j` | `--jobs` | Parallel jobs | 1 |
| | `--strict` | Exit with error if any warnings have error severity for the active evidence profile | false |
| | `--json` | Output results as JSON | false |
| | `--rights-policy` | Explicit rights policy (canonical API) | none |
| | `--preset` | Executable preset (canonical API) | none |
| | `--hidden-marker` | Hidden marker mode (canonical API) | none |
| | `--authentication` | Authentication mode (canonical API) | none |
| | `--dry-run` | Show resolved plan without processing | false |

## Normalization Architecture

All protect modes (single-file, batch sequential, batch parallel, dry-run, JSON)
share one builder: `request::build_protection_request_with_explicit_options()` →
`ProtectionRequest`. Legacy flags are translation syntax, not an independent behavior
model.

### Legacy Policy Resolution

`resolve_legacy_dmi()` applies the default DMI value when `--dmi` is omitted or set to `auto`:

- `Disabled` / `Light` → `Unspecified`
- `Standard` → `ProhibitedAiMlTraining`

Explicit `--dmi` values override the level default. Omitted `--dmi` and `--dmi auto`
are equivalent.

### Precedence Contract

General rule:

1. Explicit modern flags win when they directly specify a field.
2. Legacy flags translate only when the equivalent modern field was not explicitly supplied.
3. Contradictory explicit combinations fail with `EXIT_CONFIG` (exit code 2).
4. Defaults apply once, after explicitness is known.

`--level`/`--profile` explicitness uses clap `ValueSource`; all other legacy flags
are `Option`/bool presence. Defaulted `--level standard` / `--profile legal-notice`
never override explicit modern flags.

| Field | Modern (explicit wins) | Legacy (translates only when modern absent) | Contradiction → exit 2 |
|---|---|---|---|
| Policy | `--rights-policy` | `--dmi`, `--no-ai-training`, `--no-genai-training`, `--tdm-reserved` | Any two explicit policy sources that resolve differently |
| Channels via preset | `--preset` | `--level`, `--profile` | `--preset` with explicit `--level`/`--profile` (even when values agree) |
| Channels explicit | `--hidden-marker`, `--authentication` | `--level`, `--profile` | Explicit `--hidden-marker`/`--authentication` with explicit `--level`/`--profile` |
| Channels via preset vs explicit | `--preset` | — | `--preset` with `--hidden-marker`/`--authentication` |
| Rights metadata | (derived) | `--metadata`, `--legal-claims`, legal fields | `--metadata false` with legal fields or with a metadata-injecting preset/channels |
| Auth | `--authentication hmac` + `--key` | `--profile authenticated-provenance`/`maximal` + `--key` | HMAC without key; HMAC with `--hidden-marker disabled` |

Shared processing options (`--intensity`, `--seed`, `--format`, `--stego-redundancy`,
`--jpeg-quality`, `--progressive`, `--key`, legal metadata fields) apply identically
to both paths.

### Normalization Sequence

```
detect new-style (any --rights-policy/--preset/--hidden-marker/--authentication)
reject --preset + --level/--profile and --preset + --hidden-marker/--authentication
reject --hidden-marker/--authentication + explicit --level/--profile
build legal metadata + shorthand DMI override
resolve policy (rights-policy <- shorthand <- dmi, with conflict checks)
resolve channels (preset.to_channels() or hidden/auth explicit; legal-claims forces rights_metadata)
reject --metadata false contradictions
validate HMAC key requirements, intensity finiteness, redundancy range
attach seed/intensity/jpeg-quality/format/progressive/legal-metadata/mac-key
```

### Canonical vs Legacy Paths

- Legacy path: no `--rights-policy`, `--preset`, `--hidden-marker`, or `--authentication`
- Canonical path: any of `--rights-policy`, `--preset`, `--hidden-marker`, or `--authentication` present
- `--dry-run` does not select the canonical path; it only controls output behavior

### Conflict Rules

Conflicting expressions produce exit code 2:
- `--dmi` value contradicting `--rights-policy`
- `--no-ai-training` / `--no-genai-training` / `--tdm-reserved` contradicting `--rights-policy` or `--dmi`
- `--metadata false` with legal metadata flags
- `--metadata false` with a metadata-injecting preset/channels
- `--preset` combined with `--level` or `--profile`
- `--preset` combined with `--hidden-marker`/`--authentication`
- `--hidden-marker`/`--authentication` combined with explicit `--level`/`--profile`
- HMAC authentication without a key
- HMAC authentication with hidden marker disabled

## Input Handling

- Single file: processes and outputs to current directory or `-o` directory
- Multiple files / directory: batch mode, outputs to `-o` directory
- Output filename is always `{stem}_protected.{ext}`
- Exits with error when no input files found

## Profile Selection

The `--profile` flag selects the evidence profile:
- `legal-notice` (default): Metadata notice only. No MAC key required.
- `legal-notice-stego`: Metadata + best-effort steganography. No MAC key required.
- `authenticated-provenance`: Cryptographic payload verification. MAC key expected via `--key`.
- `maximal`: All channels. MAC key optional.

Legal metadata flags (`--copyright-notice`, etc.) auto-enable metadata injection. The profile affects which warnings are emitted, not the raw processing pipeline. Note the spelling split: legacy `--profile` takes `legal-notice-stego`, while modern `--preset` takes `legal-notice-with-stego` (clap kebab-case of `PresetArg`); `ProtectionPreset::as_str()` returns the legacy `legal-notice-stego` spelling.

## Batch Processing

When multiple inputs are provided:
- Uses rayon-based parallel processing with `-j` jobs
- Flat output to `-o` directory or current directory (does not preserve directory structure)
- Filename collision handling: `{stem}_protected_{n}.{ext}` for duplicate stems
- Progress reporting with verbose mode
- Rayon thread pool initialization fails silently if already initialized
- Single-file, sequential batch, and parallel batch all use the same `ProtectionRequest`

## Verification Mode (`--verify`)

1. Load image bytes (from `-o` output file if specified, otherwise input)
2. Call `verify_legal_notice()` which:
   - Extracts legal fields from metadata (PNG tEXt, JPEG COM, WebP)
   - Verifies steganographic payload integrity (DCT for JPEG, LSB for PNG/WebP)
   - Computes `EvidenceStrength` rating
3. Print all legal metadata fields (copyright, creator, contact, usage terms, AI constraints, DMI including canonical/legacy/conflict detail, TDM reservation, credit line, copyright owner, licensor name/email/URL, metadata date, notice-applied-at, protection seed)
4. Print stego status and authentication status
5. Print evidence strength and channels

When `--key` is provided, HMAC-SHA256 is used for stego payload verification.

## Format Auto-Detection

1. Check `--format` flag
2. Detect from input magic bytes
3. Default to PNG

## Dependencies

Production dependencies (audited Plan 087):

- `clap` 4 — Argument parsing (derive macro)
- `stegoeggo` — Library crate, default features only (no `iscc`/`conformance`/`parallel`; `signatures` feature adds `stegoeggo/signatures` + `stegoeggo/detached-manifest`)
- `rayon` — Parallel batch processing with per-file error tolerance (root `parallel` batch APIs fail fast on first error and are unsuitable for CLI batch semantics)
- `hex` — Key encoding/decoding
- `serde` / `serde_json` — `--json` and `verify-manifest --json` output
- `tempfile` — Atomic output writes via `NamedTempFile`

Removed: direct `image` (was verbose-only dimension logging; tests retain `image` as dev-dependency); root `iscc` (no CLI call sites; pulls `iscc-lib`/`blake3`/`unicode`); root `conformance` (belongs to `stegoeggo-conformance` binary, no CLI call sites; pulls `toml`/`unicode-normalization`); root `parallel` (CLI uses direct `rayon` for error-tolerant batch, never calls `process_request_bytes_parallel`).

## Modules

| File | Responsibility |
|---|---|
| `args.rs` | clap structures and value enums only |
| `request.rs` | Canonical request construction, precedence validation, legal metadata, display warnings |
| `protect.rs` | Single and batch processing orchestration, output paths, atomic writes |
| `verify.rs` | Image verification and rendering |
| `output.rs` | Human/JSON formatting types and exit-code mapping |
| `keys.rs` | Key input parsing and secret handling |
| `manifest.rs` | Feature-gated signing/detached-manifest subcommands (`signatures`) |
| `main.rs` | Thin orchestration: subcommand dispatch, verify/protect routing, dry-run/batch/single paths |

## Module Interactions

- All protection paths route through `request::build_protection_request_with_explicit_options()` → `process_request_bytes_with_warnings()` / `process_request_bytes_with_report()`. Verify uses `keys::resolve_key_input()` for all key sources (literal hex, `@file`, stdin, env `STEGOEGGO_KEY`)
- Uses `ProtectionRequest`, `RightsPolicy`, `ProtectionPreset`, `ImageOutputFormat` from `stegoeggo`
