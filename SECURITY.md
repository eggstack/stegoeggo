# Security Policy

## Supported versions

| Version | Supported |
|---|---|
| 0.4.x | Yes |
| 0.3.x | Yes |
| 0.2.x | Yes |
| Earlier than 0.2 | No |

## Reporting a vulnerability

Do not open a public GitHub issue for a security vulnerability. Contact the
maintainer using the email address in the crate metadata or GitHub profile.
Include a description, reproduction steps, and potential impact. A suggested
fix is welcome but optional.

## CLI downloads

Release binaries are not code-signed. Installers download over HTTPS and check
the SHA-256 sidecar and candidate version before installation. The sidecar is
served by the same GitHub Release as the binary, so this detects corruption or
mismatched assets but does not independently authenticate the publisher.

The updater uses the stable `stegoeggo-cli` version on crates.io to select the
matching release asset. It verifies the sidecar and candidate before replacing
the installed executable. Checksum, identity, or network failures do not trigger
a Cargo fallback. See the [installation guide](docs/installation.md) for the
full update contract.

## Image protection limits

Rights metadata can be copied, edited, or removed. Hidden markers are optional
and best effort; image transformations can damage or remove them. Neither
channel prevents scraping or proves that a model used an image.

A payload protected only by CRC32 has corruption detection, not cryptographic
authentication. HMAC can verify a marker against a shared secret, but anyone
who knows that secret can create a valid marker. HMAC does not establish
copyright ownership, signer identity, or publication time.

Use the byte-processing APIs when metadata must be written or preserved. A
`DynamicImage` round-trip works on pixels and does not preserve file-level
metadata. Same-format JPEG processing uses a byte-preserving path when the JPEG
structure is supported; generic image re-encoding can alter coefficients or
strip metadata.

## Signed manifests

The `signatures` feature is experimental. Ed25519 signatures authenticate the
signed manifest bytes, including any image digest recorded in the manifest.
They do not establish who controls a public key unless the application trusts
that key independently, and they do not establish legal ownership. Trust
evaluation is caller-owned; the library provides no implicit trust store.
