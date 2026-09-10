# Support Matrix

## Rust Toolchain

| Item | Value |
|------|-------|
| MSRV | 1.87 (stable channel) |
| Required CI toolchain | stable (currently 1.9x; runs `./scripts/check.sh` on every push/PR to `main`) |
| MSRV evidence | Weekly scheduled `Assurance` workflow pins Rust 1.87 and runs the MSRV matrix below |

The MSRV matrix (`Assurance` → `MSRV 1.87` job, Ubuntu x86_64, `--locked`) verifies:

- `cargo check -p stegoeggo-stego` (carrier, default features);
- `cargo check -p stegoeggo` (default, minimal `--no-default-features`, and `--all-features`);
- `cargo check -p stegoeggo-cli` (default and `--all-features`);
- `cargo test -p stegoeggo-stego` and `cargo test -p stegoeggo --lib --all-features`.

A dependency that breaks compilation or tests on Rust 1.87 is treated as an
explicit semver/toolchain decision: either pin a compatible dependency or
raise the declared MSRV, never a silent break.

## Supported Platforms

Evidence levels: **PR** = tested on every push/PR to `main` (standard `Check`
gate; not currently GitHub-enforced as a required status check — `main` is
unprotected, verified 2026-09-10); **Scheduled** = tested by a recurring non-blocking workflow (failure
never blocks merges); **Expected** = believed to work but with no CI evidence.

| OS | Architecture | PR | Scheduled assurance | Notes |
|----|-------------|----|---------------------|-------|
| Linux | x86_64 | Yes | Yes (weekly) | Primary development platform; standard gate runs fmt, clippy, minimal-feature check, and all-feature workspace tests |
| Linux | aarch64 | No | Yes (weekly, native `ubuntu-24.04-arm` runner) | Minimal-feature check + all-feature workspace tests |
| macOS | aarch64 | No | Yes (weekly, `macos-latest`) | Minimal-feature check + all-feature workspace tests |
| macOS | x86_64 | No | No (expected) | `macos-latest` runners are aarch64; Intel macOS is untested in CI |
| Windows | x86_64 | No | Yes (weekly, `windows-latest`) | Minimal-feature check + all-feature workspace tests |

Scheduled platform jobs replay the standard gate's compile-and-test evidence
(minus fmt/clippy, which are platform-independent) on stable Rust. See
`.github/workflows/assurance.yml` for the exact commands.

## Assurance Cadence

| Workflow | Trigger | Blocking | Proves |
|----------|---------|----------|--------|
| `CI` (`ci.yml`, `Check` job) | Every push/PR to `main` | Yes (standard gate; not currently GitHub-enforced as a required status check) | Stable compile, lint, format, and tests on Linux x86_64 |
| `Assurance` (`assurance.yml`) | Weekly + manual dispatch | No | MSRV 1.87 matrix; stable compile+tests on Linux aarch64, macOS aarch64, Windows x86_64 |
| `External Verification` (`external-verification.yml`) | Monthly + manual dispatch | No | ExifTool/xmllint/ImageMagick/libvips conformance signal |
| `Fuzz` (`fuzz.yml`) | Manual dispatch (single target) + weekly scheduled smoke (rotating 3-target subset, 120s each) | No | Parser robustness signal with crash-artifact upload |

## Supported Image Formats

| Format | Read | Write |
|--------|------|-------|
| PNG | Yes | Yes |
| JPEG | Yes | Yes |
| WebP | Yes | Yes |

## Payload Versions

| Version | Read | Write |
|---------|------|-------|
| v1 | Yes | No |
| v2 | Yes | No |
| v3 | Yes | Yes |

Write output always uses payload v3. Older payload versions are read for backward compatibility.

## Manifest Schema Versions

| Version | Read | Write |
|---------|------|-------|
| v1 | Yes | Yes |

## Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `async` | No | Tokio-based async API wrappers |
| `signatures` | No | Ed25519 signing and key management |
| `detached-manifest` | No | Detached signed manifest sidecar support |
| `iscc` | No | ISCC content identifier computation |
| `conformance` | No | Conformance harness binary and manifest parsing |
| `parallel` | No | Rayon-based parallel batch processing |
| `test-seeds` | No | Deterministic seeds for testing |
| `fuzz` | No | Fuzzing harness support |

The default feature set is empty (`default = []`).

## CLI Installation

The shipped CLI uses default features only (no `iscc`/`conformance`/`parallel`
root features, no direct `image` dependency). The `signatures` feature adds
`stegoeggo/signatures` + `stegoeggo/detached-manifest` for `keygen`/`sign`/
`verify-manifest`.

### From crates.io

```
cargo install stegoeggo-cli
```

### From source

```
git clone https://github.com/eggstack/stegoeggo
cd stegoeggo
cargo build --release --bin stegoeggo
```

## External Tools

External tools are required only for development and conformance testing. They are not required at runtime or for library use.

| Tool | Purpose |
|------|---------|
| exiftool | Metadata extraction in conformance tests |
| xmllint | XMP well-formedness validation |
| imagemagick | Image format conversion in integration tests |
| libvips | Image processing in integration tests |
