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