# Support Matrix

## Rust Toolchain

| Item | Value |
|------|-------|
| MSRV | 1.89 (stable channel) |
| Required CI toolchain | stable (currently 1.9x; runs `./scripts/check.sh` on every push/PR to `main`) |
| MSRV evidence | Weekly scheduled `Assurance` workflow pins Rust 1.89 and runs the MSRV matrix below |

The MSRV matrix (`Assurance` → `MSRV 1.89` job, Ubuntu x86_64, `--locked`) verifies:

- `cargo check -p stegoeggo-stego` (carrier, default features);
- `cargo check -p stegoeggo` (default, minimal `--no-default-features`, and `--all-features`);
- `cargo check -p stegoeggo-cli` (default and `--all-features`);
- `cargo test -p stegoeggo-stego` and `cargo test -p stegoeggo --lib --all-features`.

A dependency that breaks compilation or tests on Rust 1.89 is treated as an
explicit semver/toolchain decision: either pin a compatible dependency or
raise the declared MSRV, never a silent break.

## Supported Platforms and distribution

Evidence levels: **PR** = tested on every push/PR to `main` (standard `Check`
gate; not currently GitHub-enforced as a required status check — `main` is
unprotected, verified 2026-09-11); **Scheduled** = tested by a recurring non-blocking workflow (failure
never blocks merges); **Expected** = believed to work but with no CI evidence.

These are separate contracts. Compile/test assurance describes where CI runs;
prebuilt distribution describes where GitHub Release assets are attached; a
source-only target has no release asset but may still work through Cargo.

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

### Contract summary

| Contract | Targets |
|---|---|
| Compile/test assurance | Linux x86_64 on every PR; Linux aarch64, macOS arm64, and Windows x86_64 weekly; Rust 1.89 weekly on the MSRV matrix |
| Prebuilt binary distribution | Linux x86_64/aarch64, macOS x86_64/arm64, and Windows x86_64 |
| Source-only distribution | Other platforms and architectures, subject to Cargo/Rust and dependency support |

The macOS x86_64 binary is distributed even though it is an expected rather
than scheduled assurance target. No platform outside the binary matrix should
be assumed to have a downloadable asset.

## Assurance Cadence

| Workflow | Trigger | Blocking | Proves |
|----------|---------|----------|--------|
| `CI` (`ci.yml`, `Check` job) | Every push/PR to `main` | Yes (standard gate; not currently GitHub-enforced as a required status check) | Stable compile, lint, format, and tests on Linux x86_64 |
| `Assurance` (`assurance.yml`) | Weekly + manual dispatch | No | MSRV 1.89 matrix; stable compile+tests on Linux aarch64, macOS aarch64, Windows x86_64 |
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

The shipped CLI enables its `signatures` feature by default. It does not enable
the library's `iscc`, `conformance`, or `parallel` features and has no direct
`image` dependency. The default feature adds `stegoeggo/signatures` +
`stegoeggo/detached-manifest` for `keygen`/`sign`/`verify-manifest`.

### Prebuilt binary matrix

GitHub Releases provide these versionless executable asset names. Each asset
has a matching `<asset>.sha256` sidecar:

| OS | Architecture | Target | Asset |
|----|--------------|--------|-------|
| Linux | x86_64 | `x86_64-unknown-linux-gnu` | `stegoeggo-x86_64-unknown-linux-gnu` |
| Linux | aarch64 | `aarch64-unknown-linux-gnu` | `stegoeggo-aarch64-unknown-linux-gnu` |
| macOS | x86_64 | `x86_64-apple-darwin` | `stegoeggo-x86_64-apple-darwin` |
| macOS | arm64 | `aarch64-apple-darwin` | `stegoeggo-aarch64-apple-darwin` |
| Windows | x86_64 | `x86_64-pc-windows-msvc` | `stegoeggo-x86_64-pc-windows-msvc.exe` |

Linux GNU release binaries are built with `cargo-zigbuild` against the glibc
2.17 compatibility floor and are smoke-tested on their native release runner.
The installers fall back to Cargo only for an unsupported platform or a 404
for the binary asset. Checksum, identity, and network failures are fatal.
See [docs/installation.md](docs/installation.md) for the complete installer
and updater contract. `stegoeggo version` is offline; `stegoeggo update` uses
the stable crates.io CLI version as authority through the embedded eggfetch
transport and never invokes `sudo`.

### From crates.io

```
cargo install stegoeggo-cli --locked
```

### From source

```
git clone https://github.com/eggstack/stegoeggo
cd stegoeggo
cargo build --release --bin stegoeggo
```

## Python Binding

A typed Python frontend lives at `bindings/python/` and builds an
`abi3-py311` wheel via PyO3 + maturin. It mirrors the canonical Rust
`process_request_bytes*` and `verify_image_bytes_report` API surface and
shares the same resource-limit, error, and panic profile semantics as
the library crate. The binding is **experimental / not on PyPI**: wheels
are produced as GitHub Actions artifacts by a manually dispatched
workflow and are not published to any registry. All five documented
wheel platforms are qualified; wheel qualification evidence is recorded
in `plans/closure/language-bindings/003-status.md`.

### Documented interpreter support

| Interpreter | Range | Wheel tag |
|-------------|-------|-----------|
| CPython | 3.11, 3.12, 3.13, 3.14 | `cp311-abi3` (one wheel per OS/arch) |

Free-threaded CPython (`cp311t-abi3`, etc.) and PyPy are intentionally
unsupported in the initial milestone. The binding release profile uses
`panic = "unwind"`; the standalone CLI keeps `panic = "abort"` and is
unaffected.

### Install (local source build)

```bash
git clone https://github.com/eggstack/stegoeggo
cd stegoeggo/bindings/python
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -U pip maturin pytest
maturin develop --release
```

The wheel installs into the active virtualenv. Building from source
requires a Rust toolchain ≥ 1.89 (the binding's `rust-version` floor);
installing a prebuilt wheel will not.

### Documented platform matrix for wheel artefacts

The wheel matrix below describes what the manually-dispatched
`.github/workflows/release-python.yml` workflow produces. "Qualified"
means native build + native install + protect/verify smoke evidence is
recorded in
`plans/closure/language-bindings/003-status.md`; "configured" would mean
the matrix row is wired up in CI but the native smoke run has not yet
completed. All five rows are qualified by the
`.github/workflows/release-python.yml` run `36337194059` (head
`d40f1b37cf031b05e4f9c76af1cdfcf789d739b5`), which installed each wheel
on the same native architecture that built it and ran an import +
protect + verify smoke.

| OS | Architecture | Runner | Status |
|----|--------------|--------|--------|
| Linux | x86_64 | `ubuntu-24.04` | Qualified |
| Linux | aarch64 | `ubuntu-24.04-arm` | Qualified |
| macOS | x86_64 | `macos-15-intel` | Qualified |
| macOS | arm64 | `macos-14` | Qualified |
| Windows | x86_64 | `windows-2022` | Qualified |

A separate lightweight `.github/workflows/python-binding.yml` workflow
runs the full Python test suite on Linux x86_64 / CPython 3.11 on
binding-relevant pull requests and pushes. It builds and installs a
wheel and never publishes artifacts.

Wheels are built by the manually-dispatched
`.github/workflows/release-python.yml` workflow (no PyPI publish) and
attached as GitHub Actions artifacts. Every wheel is built on a runner
that matches its native architecture and the per-platform smoke install
runs on the same native arch. Linux cibuildwheel uses `before-all` to
install Rust 1.89 inside the build container and the manylinux
compatibility floor is pinned to `manylinux_2_28`. The same workflow
builds one sdist and the sdist job installs it through a literal
`pip install <tarball>` in a clean venv outside the source checkout.

The sdist is a self-contained PEP 517 build artefact: `pip install
stegoeggo-0.4.2.tar.gz` resolves the binding's path dependencies from
the tarball itself rather than the source checkout.

## External Tools

External tools are required only for development and conformance testing. They are not required at runtime or for library use.

| Tool | Purpose |
|------|---------|
| exiftool | Metadata extraction in conformance tests |
| xmllint | XMP well-formedness validation |
| imagemagick | Image format conversion in integration tests |
| libvips | Image processing in integration tests |
