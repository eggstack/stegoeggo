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

For warnings and full execution detail, use `process_request_bytes_with_warnings` and `process_request_bytes_with_report`.

New processing features must be expressed in `ProtectionRequest` / `ProcessingOptions` / `ProtectionChannels` first. Legacy `ProtectionContext` builders may only translate into those fields when compatibility requires it.

## Async API (`async` feature)

Use the request-based async wrappers. Each calls its synchronous canonical counterpart inside one `spawn_blocking` closure:

```rust
use stegoeggo::{process_request_bytes_async, ProtectionRequest, RightsNotice, RightsPolicy};

async fn protect(input: Vec<u8>) -> Result<Vec<u8>, stegoeggo::Error> {
    let request = ProtectionRequest::with_hidden_marker(
        RightsNotice::new(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_seed(42);
    process_request_bytes_async(input, request).await
}
```

`process_request_bytes_with_warnings_async` and `process_request_bytes_with_report_async` are the warnings/report equivalents. The `parallel` batch variants (`process_request_bytes_parallel_async`, `..._with_warnings_parallel_async`, `..._with_report_parallel_async`) run the whole batch on one blocking thread via the synchronous Rayon batch.

## Parallel batch API (`parallel` feature)

One shared request applied to many byte buffers, preserving input order. There is no second batch executor and no per-item request form:

```rust
use stegoeggo::{process_request_bytes_parallel, ProtectionRequest, RightsNotice, RightsPolicy};

let images: Vec<Vec<u8>> = vec![std::fs::read("a.png")?, std::fs::read("b.png")?];
let request = ProtectionRequest::with_hidden_marker(
    RightsNotice::new(),
    RightsPolicy::ProhibitedAiMlTraining,
)
.with_seed(42);
let outputs = process_request_bytes_parallel(&images, &request)?;
```

`process_request_bytes_with_warnings_parallel` and `process_request_bytes_with_report_parallel` are the warnings/report equivalents.

## Byte APIs versus `DynamicImage`

This distinction matters: file metadata lives in the encoded image container. APIs that accept and return `image::DynamicImage` operate on decoded pixels and cannot preserve or inject file-level metadata by themselves. Use `process_request_bytes` (canonical) or the legacy `process_image_bytes` path when the resulting file must contain rights metadata.

| Function | Input/Output | Metadata preserved |
|----------|-------------|-------------------|
| `process_request_bytes` | `&[u8]` → `Vec<u8>` | Yes |
| `process_image_bytes` | `&[u8]` → `Vec<u8>` | Yes |
| `process_image` | `DynamicImage` → `DynamicImage` | No (pixels only) |

## Verification

`verify_image_bytes_report` is the canonical verification operation for rich
integrations. It performs one rights parse plus one hidden-marker search and
returns `VerificationReport`:

```rust
use stegoeggo::verify_image_bytes_report;

let report = verify_image_bytes_report(&output_bytes, &[]);
println!("{:?}", report.hidden_marker().status());
```

`verify_image_bytes` (`VerificationStatus`), `verify_image_bytes_detailed`
(`VerificationResult`), and `verify_legal_notice` (`NoticeVerification`) are
compatibility projections derived from the same canonical facts.
`VerificationStatus` remains stable and is not deprecated: it reports
hidden-marker integrity only (`Verified`/`Invalid`/`NotFound`), while
`VerificationReport::summary_status` upgrades metadata-only evidence to
`Verified` for overall assessment.

The report distinguishes metadata-only notices, best-effort steganographic evidence, and HMAC-authenticated provenance when a matching key is supplied.

Verification should be interpreted as evidence about what is present in the file, not as a legal conclusion. Metadata can be copied or forged; an HMAC proves knowledge of a secret key, not ownership of the underlying work.

## Compatibility surface

The older `ProtectionContext`, `ProtectionLevel`, `EvidenceProfile`, `with_dmi()`, and related level/context sync, async, and parallel wrappers remain functional compatibility adapters but are deprecated for new code. They translate once into `ProtectionRequest` and delegate to the canonical path, adding only compatibility presentation warnings (`MissingMacKey`, `ContradictoryLegalClaims`, `JpegReencodeFragile`) where applicable. `ProtectionPipeline` (stateless adapter) and the context/level-based `Protector` trait are likewise retained through 0.x with removal at the v1 boundary (no mechanical rename). See [DEPRECATIONS.md](../DEPRECATIONS.md), [migration-v0.3.md](migration-v0.3.md), and [096-status.md](../plans/096-status.md).

## Examples

See [`examples/`](https://github.com/eggstack/stegoeggo/blob/main/examples) for complete working examples:

- `protect_and_verify.rs` — Full pipeline: protect an image and verify the protection
- `legal_metadata.rs` — Legal metadata injection with copyright and usage terms
- `generic_stego.rs` — Raw, in-place, framed, tiled, strict-JPEG, prepared-JPEG, and borrowed-view carrier operations via `stegoeggo_stego` directly
- `verify_saved.rs` — Verify an already-protected image file
