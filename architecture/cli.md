# CLI Tool

**Source:** `stegoeggo-cli/src/main.rs` (~848 lines)

Command-line interface for `stegoeggo`. Built with `clap` 4 (derive).

## Binary

```bash
stegoeggo [OPTIONS] <INPUT>...
```

## Options

| Flag | Long | Description | Default |
|------|------|-------------|---------|
| `-o` | `--output` | Output directory (for batch) or output file (for single) | current directory |
| `-V` | `--verify` | Verify protection signature | false |
| `-l` | `--level` | Protection level | standard |
| `-p` | `--profile` | Evidence profile | legal-notice |
| `-i` | `--intensity` | Float 0.0–1.0 | 0.5 |
| `-s` | `--seed` | Seed for reproducibility | random |
| `-f` | `--format` | Output format (png/jpg/webp) | auto |
| | `--stego-redundancy` | 1–10 | 2 |
| | `--jpeg-quality` | 1–100 | 90 |
| | `--progressive` | Progressive JPEG | false |
| `-v` | `--verbose` | Verbose output | false |
| `-d` | `--dmi` | DMI metadata value | auto |
| | `--metadata` | Inject metadata (None = use level default) | None |
| | `--legal-claims` | Inject legal claims | false |
| | `--copyright-holder` | Copyright holder name | none |
| | `--creator` | Creator/author name | none |
| | `--contact` | Contact email or URL | none |
| | `--rights-url` | URL to full usage terms | none |
| | `--usage-terms` | Brief usage terms summary | none |
| | `--ai-constraints` | AI-specific constraints text | none |
| | `--no-ai-training` | Prohibit AI/ML training (DMI preset) | false |
| | `--no-genai-training` | Prohibit generative AI training (DMI preset) | false |
| | `--tdm-reserved` | Reserve TDM rights (DMI preset) — legacy; no longer emitted in image metadata | false |
| `-k` | `--key` | Hex cryptographic key | none |
| `-j` | `--jobs` | Parallel jobs | 1 |
| | `--strict` | Exit with error if any warnings have error severity for the active evidence profile | false |

## Input Handling

- Single file: processes and outputs to current directory or `-o` directory
- Multiple files / directory: batch mode, outputs to `-o` directory
- Output filename is always `{stem}_protected.{ext}`
- Exits with error when no input files are found

## Profile Selection

The `--profile` flag selects the evidence profile:
- `legal-notice` (default): Metadata notice only. No MAC key required.
- `legal-notice-stego`: Metadata + best-effort steganography. No MAC key required.
- `authenticated-provenance`: Cryptographic payload verification. MAC key expected via `--key`.
- `maximal`: All channels. MAC key optional.

Legal metadata flags (`--copyright-holder`, etc.) auto-enable metadata injection. The profile affects which warnings are emitted, not the raw processing pipeline.

## Batch Processing

When multiple inputs are provided:
- Uses rayon-based parallel processing with `-j` jobs
- Flat output to `-o` directory or current directory (does not preserve directory structure)
- Filename collision handling: `{stem}_protected_{n}.{ext}` for duplicate stems
- Progress reporting with verbose mode
- Rayon thread pool initialization fails silently if already initialized

## Verification Mode (`-V`)

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

- **lib.rs**: Calls `process_image_bytes`, `process_images_bytes_parallel`, `verify_image_bytes`
- **types.rs**: Uses `ProtectionLevel`, `ProtectionContext`, `ImageOutputFormat`, `DmiValue`
