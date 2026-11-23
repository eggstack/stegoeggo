# Rust API

## Protect encoded image bytes

The canonical interface is `ProtectionRequest` with `RightsPolicy`. Use the
byte API when the output must contain rights metadata:

~~~rust
use stegoeggo::{process_request_bytes, ProtectionRequest, RightsNotice, RightsPolicy};

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
    std::fs::write("protected.png", output)?;
    Ok(())
}
~~~

To request a best-effort hidden marker as well as metadata, use
`ProtectionRequest::with_hidden_marker(notice, policy)`. The marker may not
survive transformations such as resizing, cropping, or lossy re-encoding.

For repeatable output, set both a seed and a timestamp override on the request:

~~~rust
let request = ProtectionRequest::metadata_only(notice, policy)
    .with_seed(42)
    .with_timestamp_override("2026-01-01T00:00:00Z");
~~~

## Byte APIs and pixel APIs

Image metadata belongs to the encoded file container. A `DynamicImage` holds
pixels only.

| Function | Input and output | Writes file metadata |
|---|---|---|
| `process_request_bytes` | `&[u8] → Vec<u8>` | Yes |
| `process_image_bytes` | `&[u8] → Vec<u8>` | Yes |
| `process_image` | `DynamicImage → DynamicImage` | No, pixels only |

The byte APIs are the recommended path for new applications. The older
`process_image_bytes` function remains available as a compatibility API.

## Verification

Use `verify_image_bytes_report` for per-channel detail:

~~~rust
use stegoeggo::verify_image_bytes_report;

let report = verify_image_bytes_report(&output_bytes, &[]);
println!("{:?}", report.hidden_marker().status());
~~~

`VerificationReport` includes metadata and hidden-marker evidence.
`VerificationStatus` is a simpler hidden-marker integrity result. A valid
metadata-only notice can count as overall evidence in the report even when no
hidden marker is present.

Verification describes evidence in the file, not whether a legal claim is
valid. HMAC verifies a shared-secret match, not ownership, public identity, or
a trusted timestamp.

## Optional features

The root library disables optional features by default.

- Enable `async` for request-based async wrappers such as
  `process_request_bytes_async` and `process_request_bytes_with_report_async`.
- Enable `parallel` for order-preserving byte batches such as
  `process_request_bytes_parallel`. One request is applied to the entire batch.
- Enable `signatures` and `detached-manifest` for signing and manifest APIs.

See the [feature list](../SUPPORT.md#optional-library-features) for the full
set. The CLI package enables `signatures` by default.

## Compatibility APIs and examples

`ProtectionContext`, `ProtectionLevel`, and `EvidenceProfile` remain available
as compatibility adapters during 0.x. New code should use
`ProtectionRequest` and `RightsPolicy`. See
[DEPRECATIONS.md](../DEPRECATIONS.md) and the
[migration guide](migration-v0.3.md).

Complete examples are in [`examples/`](https://github.com/eggstack/stegoeggo/blob/main/examples):
`protect_and_verify.rs`, `legal_metadata.rs`, `generic_stego.rs`, and
`verify_saved.rs`.
