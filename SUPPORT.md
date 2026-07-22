# Support Matrix

## Rust Toolchain

| Item | Value |
|------|-------|
| MSRV | 1.87 (stable channel) |
| Tested stable | 1.87+ |

## Supported Platforms

| OS | Architecture | CI Tested | Notes |
|----|-------------|-----------|-------|
| Linux | x86_64 | Yes | Primary development platform |
| Linux | aarch64 | No | May work, untested in CI |
| macOS | x86_64 | No | May work, untested in CI |
| macOS | aarch64 | No | May work, untested in CI |
| Windows | x86_64 | No | May work, untested in CI |

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
| `test-seeds` | No | Deterministic seeds for testing |
| `fuzz` | No | Fuzzing harness support |

The default feature set is empty (`default = []`).

## CLI Installation

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
