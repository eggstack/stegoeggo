# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.3.x   | :white_check_mark: |
| 0.2.x   | :white_check_mark: |
| < 0.2   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability within stegoeggo, please report it responsibly.

Do NOT open a public GitHub issue. Instead, contact the maintainer directly at the email address listed in the crate metadata or GitHub profile.

Please include:
- A description of the vulnerability
- Steps to reproduce
- Potential impact assessment
- Optional: suggested fix

## Security Considerations

### Without a MAC key
Steganographic payload verification uses a non-cryptographic CRC32 checksum. An attacker who can read the image bytes can forge valid-looking payloads. For production deployments (e.g., CDN protection against malicious scrapers), **always set a MAC key** via `.with_mac_key()`.

### Metadata stripping
All image metadata (XMP, IPTC, EXIF, tEXt chunks) can be stripped by any image processing tool. The steganographic layer provides a secondary evidence channel. For maximum legal deterrence, serve protected images byte-identical and reference their ISCC code.

### JPEG re-encoding
The JPEG fast path (JPEG-in/JPEG-out through `process_image_bytes`) preserves DCT coefficients and quantization tables. Re-encoding through standard image libraries (e.g., the `image` crate's encoder) rebuilds Q-tables from scratch and discards COM/APP1 markers, destroying the protection evidence. Always use `process_image_bytes` for JPEG processing to preserve protection data.

### Ed25519 signing (experimental)

The `signatures` feature provides Ed25519 signing via `ed25519-dalek`, a well-audited crate implementing the standard Ed25519 elliptic-curve signature scheme. This is real Ed25519 — not a custom or homegrown construction.

Key security properties:
- **Deterministic signatures**: Same key + same message always produces the same signature (RFC 8032 compliant).
- **Private key zeroization**: `SigningKey` zeroizes secret bytes on drop and exposes a `zeroize()` method.
- **No private key serialization**: `SigningKey` intentionally does not implement `Serialize` to prevent accidental key material leakage.
- **Constant-time verification**: Signature verification uses `ed25519_dalek::Verifier`, which is constant-time.

What a valid signature proves:
- The holder of the private key that corresponds to the embedded public key signed the canonical claim bytes.
- The claim bytes have not been altered since signing.

What a valid signature does **not** prove:
- Copyright ownership or authorship.
- That the signer is a trusted party.
- That the image has not been tampered with since signing (signatures bind claim bytes, not the image pixel data).

The `signatures` feature is **experimental**. API surfaces within the signing module may change without notice between minor releases. Trust evaluation is caller-owned — the library ships no implicit trust store.