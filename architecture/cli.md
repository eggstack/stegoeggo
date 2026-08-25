# CLI Tool

**Source:** `stegoeggo-cli/src/main.rs` (~2487 lines)

Command-line interface for `stegoeggo`. Built with `clap` 4 (derive). Routes all protection through `ProtectionRequest`.

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
| `-k` | `--key` | Hex cryptographic key | none |
| | `--known-seeds` | Additional seeds to try during verification (comma-separated u64) | none |
| `-j` | `--jobs` | Parallel jobs | 1 |
| | `--strict` | Exit with error if any warnings have error severity for the active evidence profile | false |
| | `--json` | Output results as JSON | false |
| | `--rights-policy` | Explicit rights policy (canonical API) | none |
| | `--preset` | Executable preset (canonical API) | none |
| | `--hidden-marker` | Hidden marker mode (canonical API) | none |
| | `--authentication` | Authentication mode (canonical API) | none |
| | `--dry-run` | Show resolved plan without processing | false |

## Normalization Architecture

All protection routes through `build_protection_request()` → `ProtectionRequest`. Legacy arguments (`--level`, `--profile`, `--dmi`, shorthands) are compatibility syntax translated into the canonical request model.

### Legacy Policy Resolution

`resolve_legacy_dmi()` applies the default DMI value when `--dmi` is omitted or set to `auto`:

- `Disabled` / `Light` → `Unspecified`
- `Standard` → `ProhibitedAiMlTraining`

Explicit `--dmi` values override the level default.

### Normalization Sequence

```
base legacy level default (via resolve_legacy_dmi)
explicit --dmi, if not auto
legal shorthand override (--no-ai-training, --no-genai-training, --tdm-reserved), if present
conflict check against canonical explicit rights policy
conversion to RightsPolicy
```

### Canonical vs Legacy Paths

- Legacy path: `--level`, `--profile`, `--dmi` (no `--rights-policy`, `--preset`, `--hidden-marker`, or `--authentication`)
- Canonical path: any of `--rights-policy`, `--preset`, `--hidden-marker`, or `--authentication` present
- `--dry-run` does not select the canonical path; it only controls output behavior

### Conflict Rules

Conflicting expressions produce exit code 2:
- `--dmi` value contradicting `--rights-policy`
- `--no-ai-training` / `--no-genai-training` / `--tdm-reserved` contradicting `--rights-policy`
- `--metadata false` with legal metadata flags
- `--preset` combined with `--level` or `--profile`
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

Legal metadata flags (`--copyright-notice`, etc.) auto-enable metadata injection. The profile affects which warnings are emitted, not the raw processing pipeline.

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
3. Print legal fields (copyright, creator, contact, usage terms, AI constraints, DMI)
4. Print stego status and authentication status
5. Print evidence strength and channels

When `--key` is provided, HMAC-SHA256 is used for stego payload verification.

## Format Auto-Detection

1. Check `--format` flag
2. Detect from input magic bytes
3. Default to PNG

## Dependencies

- `clap` 4 — Argument parsing (derive macro)
- `stegoeggo` — Library crate
- `image` — Image loading for verbose reporting
- `rayon` — Parallel batch processing
- `hex` — Key encoding

## Module Interactions

- **lib.rs**: All protection paths route through `build_protection_request()` → `process_request_bytes()` / `process_request_bytes_with_warnings()`. Verify uses `resolve_key_input()` for all key sources (literal hex, `@file`, stdin, env `STEGOEGGO_KEY`)
- **types.rs**: Uses `ProtectionRequest`, `RightsPolicy`, `ProtectionPreset`, `ImageOutputFormat`
