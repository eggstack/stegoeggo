# CLI Tool

**Source:** `cloakrs-cli/src/main.rs` (~545 lines)

Command-line interface for `cloakrs`. Built with `clap` 4 (derive).

## Binary

```bash
cloakrs [OPTIONS] <INPUT>...
```

## Options

| Flag | Long | Description | Default |
|------|------|-------------|---------|
| `-o` | `--output` | Output file (single) or directory (batch) | stdin/stdout |
| `-V` | `--verify` | Verify protection signature | false |
| `-l` | `--level` | Protection level | standard |
| `-i` | `--intensity` | Float 0.0–1.0 | 0.5 |
| `-s` | `--seed` | Seed for reproducibility | random |
| `-f` | `--format` | Output format (png/jpg/webp) | auto |
| | `--stego-redundancy` | 1–5 | 2 |
| | `--jpeg-quality` | 1–100 | 90 |
| | `--progressive` | Progressive JPEG | false |
| `-v` | `--verbose` | Verbose output | false |
| `-d` | `--dmi` | DMI metadata value | auto |
| | `--metadata` | Inject metadata | false |
| | `--legal-claims` | Inject legal claims | false |
| `-k` | `--key` | Hex cryptographic key | none |
| `-j` | `--jobs` | Parallel jobs | num_cpus |

## Input Handling

- Single file: processes and outputs to stdout or `-o` file
- Multiple files / directory: batch mode, outputs to `-o` directory
- Reads from stdin when no input files specified

## Batch Processing

When multiple inputs are provided:
- Uses rayon-based parallel processing with `-j` jobs
- Preserves directory structure in output
- Progress reporting with verbose mode

## Verification Mode (`-V`)

1. Load image bytes
2. Extract seed from metadata (PNG tEXt, JPEG COM, WebP META)
3. Check DCT stego for JPEG images
4. Report protection details (level, seed, intensity)
5. Verify HMAC signature if key provided

## Format Auto-Detection

1. Check `--format` flag
2. Check output file extension
3. Detect from input magic bytes
4. Default to input format

## Dependencies

- `clap` 4 — Argument parsing (derive macro)
- `cloakrs` — Library crate
- `image` — Image loading for verbose reporting
- `rayon` — Parallel batch processing
- `hex` — Key encoding

## Module Interactions

- **lib.rs**: Calls `process_image_bytes`, `process_images_bytes_parallel`, `verify_image_bytes`
- **types.rs**: Uses `ProtectionLevel`, `ProtectionContext`, `ImageOutputFormat`, `DmiValue`
