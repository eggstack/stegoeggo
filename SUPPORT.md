# Support

## Rust, CLI, and platforms

| Item | Support |
|---|---|
| Minimum Rust version | 1.89 stable |
| CLI prebuilt targets | Linux x86_64 and aarch64, macOS x86_64 and arm64, Windows x86_64 |
| Other targets | May build from source when Rust and dependencies support them; not included in the prebuilt matrix |

The CLI installers, release asset names, and update behavior are documented in
[the installation guide](docs/installation.md). The library crate has no
features enabled by default. The CLI package enables `signatures` by default.

## Image formats

| Format | Rights metadata | Hidden marker |
|---|---|---|
| PNG | Supported | Best-effort pixel-domain LSB |
| JPEG | Supported | Best-effort DCT embedding on supported JPEG structures |
| WebP | Supported | Best-effort raster embedding for supported lossless images |

The output format determines the hidden-marker carrier. Lossy or animated WebP
is not supported by the WebP carrier. A marker is optional and may be damaged
by re-encoding, resizing, cropping, or other image transformations. See
[format support](docs/formats.md) for details.

## Optional library features

All root-library features are disabled by default.

| Feature | Purpose |
|---|---|
| `async` | Tokio-based request processing wrappers |
| `parallel` | Rayon-based byte batch processing |
| `signatures` | Ed25519 signing and key management |
| `detached-manifest` | Detached manifest support |
| `iscc` | Content identifier computation |
| `conformance` | Conformance harness and manifest parsing |
| `webp` | Still-lossless WebP carrier API |

The CLI enables `signatures` by default, including the `keygen`, `sign`, and
`verify-manifest` commands. See [CLI installation](docs/installation.md) and
[CLI usage](docs/cli-usage.md).

## Language bindings

| Binding | Status |
|---|---|
| C | ABI v1 is stable. See [the C binding guide](bindings/c/README.md) and [ABI contract](bindings/c/ABI-V1.md). |
| Python | Experimental and not published to PyPI. See [the Python guide](bindings/python/README.md). |
| Node.js | Experimental and not published to npm. See [the Node.js guide](bindings/node/README.md). |
