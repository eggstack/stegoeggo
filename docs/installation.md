# Install the CLI

Prebuilt binaries are the simplest way to install StegoEggo. Rust is not
required on the supported targets.

## Install a prebuilt binary

On supported Unix systems:

~~~bash
curl -fsSL https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh | bash
~~~

The installer detects Linux x86_64/aarch64 and macOS x86_64/arm64, downloads
the matching asset, verifies its SHA-256 sidecar, checks the binary identity,
and installs it. A root install uses `/usr/local/bin`; a user install uses
`$HOME/.local/bin`. The installer does not edit shell startup files or invoke
`sudo`. Add the install directory to `PATH` if needed.

On Windows PowerShell, install the x86_64 binary with:

~~~powershell
irm https://github.com/eggstack/stegoeggo/releases/latest/download/install.ps1 | iex
~~~

It installs to `%LOCALAPPDATA%\StegoEggo\bin` and does not edit `PATH`.

## Supported release targets

Each executable has a matching `.sha256` sidecar. Release asset names are
maintained in [`release/eggpack/distribution.toml`](../release/eggpack/distribution.toml).

| Target | Asset |
|---|---|
| `x86_64-unknown-linux-gnu` | `stegoeggo-x86_64-unknown-linux-gnu` |
| `aarch64-unknown-linux-gnu` | `stegoeggo-aarch64-unknown-linux-gnu` |
| `x86_64-apple-darwin` | `stegoeggo-x86_64-apple-darwin` |
| `aarch64-apple-darwin` | `stegoeggo-aarch64-apple-darwin` |
| `x86_64-pc-windows-msvc` | `stegoeggo-x86_64-pc-windows-msvc.exe` |

Linux binaries target glibc 2.17. The release feature set includes `signatures`,
which provides `keygen`, `sign`, and `verify-manifest`. Release binaries are not
code-signed, and macOS binaries are not notarized. Depending on local policy,
Gatekeeper or SmartScreen may require an override.

The SHA-256 sidecar detects corruption or an asset mismatch. Because it is
served from the same GitHub Release as the binary, it is not an independent
publisher signature.

## Pin a version

The Unix installer accepts an exact release version:

~~~bash
curl -fsSL https://github.com/eggstack/stegoeggo/releases/download/v0.4.2/install.sh | bash -s -- --version 0.4.2
~~~

The PowerShell installer accepts `-Version`:

~~~powershell
& ([scriptblock]::Create((irm https://github.com/eggstack/stegoeggo/releases/latest/download/install.ps1))) -Version 0.4.2
~~~

An unpinned install uses the latest stable release. Checksum, identity, and
network failures stop installation. Unsupported targets and a missing release
asset (HTTP 404) may fall back to Cargo when it is available.

## Update

Check the installed version offline:

~~~bash
stegoeggo version
~~~

Update to the latest stable CLI release:

~~~bash
stegoeggo update
~~~

The updater selects the stable `stegoeggo-cli` version from crates.io, then
checks the matching GitHub Release binary and checksum before replacing the
installed executable. It ignores prereleases. Network, checksum, or binary
validation failures do not trigger a Cargo fallback. Cargo is used only for an
unsupported target or an HTTP 404 for the exact release asset. In that case,
Cargo installs the selected stable CLI version. See the [security
policy](../SECURITY.md) for the limits of the checksum check.

## Install with Cargo

Use Cargo when you are on an unsupported target or prefer a source build:

~~~bash
cargo install stegoeggo-cli --locked
~~~

The CLI package enables `signatures` by default. To build from a checkout:

~~~bash
cargo build --locked --release --package stegoeggo-cli --bin stegoeggo
~~~

After a successful self-update, the executable path is binary-managed until
Cargo installs it again.

## Uninstall

Remove the executable at its install path:

~~~bash
rm "$HOME/.local/bin/stegoeggo"
~~~

For a root Unix install, remove `/usr/local/bin/stegoeggo`. On Windows, remove
`%LOCALAPPDATA%\StegoEggo\bin\stegoeggo.exe`. Remove the directory from
`PATH` if you added it. The installer does not register a package-manager
uninstall entry.
