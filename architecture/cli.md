# CLI Tool

**Source:** `stegoeggo-cli/src/main.rs` (~545 lines)

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
| `-k` | `--key` | Hex cryptographic key | none |
| `-j` | `--jobs` | Parallel jobs | 1 |

## Input Handling

- Single file: processes and outputs to current directory or `-o` directory
- Multiple files / directory: batch mode, outputs to `-o` directory
- Output filename is always `{stem}_protected.{ext}`
- Exits with error when no input files are found

## Batch Processing

When multiple inputs are provided:
- Uses rayon-based parallel processing with `-j` jobs
- Flat output to `-o` directory or current directory (does not preserve directory structure)
- Filename collision handling: `{stem}_protected_{n}.{ext}` for duplicate stems
- Progress reporting with verbose mode
- Rayon thread pool initialization fails silently if already initialized

## Verification Mode (`-V`)

1. Load image bytes
2. Extract seed from metadata (PNG tEXt, JPEG COM, WebP META)
3. If seed found: report it and print protection details
4. If no seed: fall back to LSB stego payload extraction (pixel stego)
5. Report protection details (level, seed, intensity) from extracted payload
6. No HMAC key handling in verify path — verification is informational only

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
