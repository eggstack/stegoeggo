# CLI Installation

## Prebuilt binaries

The preferred CLI installation does not require Rust on supported Unix
platforms:

```bash
curl -fsSL https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh | bash
```

The installer detects Linux x86_64/aarch64 and macOS x86_64/arm64, downloads
the matching release asset, verifies its SHA-256 sidecar, validates the
candidate's `stegoeggo X.Y.Z` identity, and then installs it.

On Unix, a root install goes to `/usr/local/bin/stegoeggo`. A non-root install
goes to `$HOME/.local/bin/stegoeggo`; the installer creates that directory but
does not edit shell startup files. Add it to `PATH` yourself if the installer
warns that it is absent. The installer never invokes `sudo`.

Windows PowerShell users can install the x86_64 binary with:

```powershell
irm https://github.com/eggstack/stegoeggo/releases/latest/download/install.ps1 | iex
```

The Windows installer uses `%LOCALAPPDATA%\StegoEggo\bin\stegoeggo.exe`, does
not edit `PATH`, and warns when that directory is not already on `PATH`.

To pin a PowerShell install without first saving the script, invoke the
downloaded script block with its `-Version` parameter:

```powershell
& ([scriptblock]::Create((irm https://github.com/eggstack/stegoeggo/releases/latest/download/install.ps1))) -Version 0.4.0
```

## Supported binary assets

Every executable has a matching `<asset>.sha256` sidecar. The stable asset
contract is maintained in [`scripts/release-targets.txt`](../scripts/release-targets.txt):

| Target | Asset |
|---|---|
| `x86_64-unknown-linux-gnu` | `stegoeggo-x86_64-unknown-linux-gnu` |
| `aarch64-unknown-linux-gnu` | `stegoeggo-aarch64-unknown-linux-gnu` |
| `x86_64-apple-darwin` | `stegoeggo-x86_64-apple-darwin` |
| `aarch64-apple-darwin` | `stegoeggo-aarch64-apple-darwin` |
| `x86_64-pc-windows-msvc` | `stegoeggo-x86_64-pc-windows-msvc.exe` |

Linux GNU binaries use the cargo-zigbuild glibc 2.17 compatibility target.
The distributed feature set is the CLI package default, `signatures`, so
prebuilt and Cargo-installed binaries include `keygen`, `sign`, and
`verify-manifest`.

## Pinned versions and fallback

Pin a Unix install to an exact release version:

```bash
curl -fsSL https://github.com/eggstack/stegoeggo/releases/download/v0.4.0/install.sh | bash -s -- --version 0.4.0
```

The pinned installer downloads from that exact tag. An unpinned installer uses
the GitHub `releases/latest/download` endpoint. For either mode, checksum
failure, candidate identity failure, and network failure stop the install;
they never trigger a source fallback. Unsupported platforms and a missing
binary asset (HTTP 404) may fall back to Cargo when it is installed.

## Updating an installation

Check the installed version without network access:

```bash
stegoeggo version
```

Update from the latest stable release:

```bash
stegoeggo update
```

The updater treats the published `stegoeggo-cli` version on crates.io as the
release authority. It ignores prereleases, downloads the exact matching
version-tagged GitHub Release asset for the host target, verifies its SHA-256
sidecar, and validates `stegoeggo X.Y.Z` before replacing the executable.
Network, checksum, candidate identity, and server errors are fatal; Cargo is
used only for an unsupported target or an HTTP 404 for the exact binary asset.

The destination directory must be writable before any download starts. The
updater never invokes `sudo`; for a root-owned `/usr/local/bin/stegoeggo`, run
the update with appropriate privileges or use the bootstrap installer.

An update replaces the executable at its current path. If that path came from
Cargo, it becomes binary-managed until a later `cargo install` replaces it
again. On unsupported targets or a missing release asset, the Cargo fallback
installs the exact stable CLI version through Cargo instead of changing Cargo's
package metadata.

## Cargo fallback

Cargo is the supported source-install fallback:

```bash
cargo install stegoeggo-cli --locked
```

The CLI package enables `signatures` by default, matching the prebuilt
feature set. Build directly from a checkout with:

```bash
cargo build --locked --release --package stegoeggo-cli --bin stegoeggo
```

## Release verification

Maintainers can validate a tagged checkout before dispatching the manual
binary workflow:

```bash
./scripts/release-binary-preflight.sh --tag=vX.Y.Z
```

After downloading or assembling release assets, validate all executables and
sidecars with:

```bash
./scripts/release-check-assets.sh --dir=release-assets --version=X.Y.Z
```

The release workflow attaches GitHub Release assets only. It does not publish
crates.io packages; crate publication remains the manual carrier → library →
CLI sequence described in [`RELEASING.md`](../RELEASING.md).
