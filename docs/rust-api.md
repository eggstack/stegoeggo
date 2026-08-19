# Rust API

## Canonical interface

The canonical library interface is `ProtectionRequest` + `RightsPolicy`. For metadata that must remain in the encoded file, use the byte APIs such as `process_request_bytes`.

```rust
use stegoeggo::{
    process_request_bytes, ProtectionRequest, RightsNotice, RightsPolicy,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::fs::read("image.png")?;

    let notice = RightsNotice::new()
        .with_copyright_holder("Example Artist")
        .with_creator("Example Artist")
        .with_usage_terms("No AI/ML training.")
        .with_web_statement_of_rights("https://example.com/rights");

    let request = ProtectionRequest::metadata_only(
        notice,
        RightsPolicy::ProhibitedAiMlTraining,
    );

    let output = process_request_bytes(&input, &request)?;
    std::fs::write("image_protected.png", output)?;
    Ok(())
}
```

To request the hidden marker instead:

```rust
let request = ProtectionRequest::with_hidden_marker(
    notice,
    RightsPolicy::ProhibitedAiMlTraining,
);
```

The older `ProtectionContext`, `ProtectionLevel`, `EvidenceProfile`, `with_dmi()`, and related APIs remain functional compatibility surfaces but are deprecated for new code. See [DEPRECATIONS.md](../DEPRECATIONS.md) and [migration-v0.3.md](migration-v0.3.md).

## Byte APIs versus `DynamicImage`

This distinction matters: file metadata lives in the encoded image container. APIs that accept and return `image::DynamicImage` operate on decoded pixels and cannot preserve or inject file-level metadata by themselves. Use `process_request_bytes` (canonical) or the legacy `process_image_bytes` path when the resulting file must contain rights metadata.

| Function | Input/Output | Metadata preserved |
|----------|-------------|-------------------|
| `process_request_bytes` | `&[u8]` → `Vec<u8>` | Yes |
| `process_image_bytes` | `&[u8]` → `Vec<u8>` | Yes |
| `process_image` | `DynamicImage` → `DynamicImage` | No (pixels only) |

## Verification

```rust
use stegoeggo::verify_image_bytes;

let report = verify_image_bytes(&output_bytes, &[]);
println!("{:?}", report);
```

The report distinguishes metadata-only notices, best-effort steganographic evidence, and HMAC-authenticated provenance when a matching key is supplied.

Verification should be interpreted as evidence about what is present in the file, not as a legal conclusion. Metadata can be copied or forged; an HMAC proves knowledge of a secret key, not ownership of the underlying work.

## Examples

See [`examples/`](https://github.com/eggstack/stegoeggo/blob/main/examples) for complete working examples:

- `protect_and_verify.rs` — Full pipeline: protect an image and verify the protection
- `legal_metadata.rs` — Legal metadata injection with copyright and usage terms
- `generic_stego.rs` — Raw, in-place, and framed carrier operations
- `verify_saved.rs` — Verify an already-protected image file
