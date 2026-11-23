# Support

## Rust, CLI, and platforms

| Item | Support |
|---|---|
| Minimum Rust version | 1.89 stable |
| Prebuilt CLI targets | Linux x86_64/aarch64, macOS x86_64/arm64, Windows x86_64 |
| Other targets | May build from source if Rust and dependencies support them |

See the [installation guide](docs/installation.md) for installers, release
assets, and update behavior. The root library has no default features. The CLI
package enables `signatures` by default.

## Image formats

| Format | Rights metadata | Hidden marker |
|---|---|---|
| PNG | Supported | Best-effort pixel LSB |
| JPEG | Supported | Best-effort DCT embedding on supported JPEG structures |
| WebP | Supported | Best-effort pixel LSB; encoded losslessly |

The output format determines the marker carrier. Re-encoding, resizing, or
cropping can damage a marker. The separate `stegoeggo-stego` WebP carrier
feature accepts still lossless VP8L input and rejects lossy or animated WebP.
The root byte-processing pipeline does not route to that optional carrier
implicitly. See [format support](docs/formats.md) for details.

## Optional library features

All root-library features are disabled by default.

| Feature | Purpose |
|---|---|
| `async` | Tokio-based request-processing wrappers |
| `parallel` | Rayon-based byte batch processing |
| `signatures` | Ed25519 signing and key management |
| `detached-manifest` | Detached manifest support |
| `iscc` | Content identifier computation |
| `conformance` | Conformance harness and manifest parsing |
| `webp` | Still-lossless WebP carrier API |

The CLI enables `signatures` by default, including `keygen`, `sign`, and
`verify-manifest`. See the [CLI guide](docs/cli-usage.md).

## Language bindings

| Binding | Status |
|---|---|
| C | ABI v1 is stable. See the [C guide](bindings/c/README.md) and [ABI contract](bindings/c/ABI-V1.md). |
| Python | Experimental and not published to PyPI. See the [Python guide](bindings/python/README.md). |
| Node.js | Experimental and not published to npm. See the [Node.js guide](bindings/node/README.md). |
