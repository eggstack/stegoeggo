# stegoeggo Python bindings

Python frontend for the [`stegoeggo`](https://docs.rs/stegoeggo) Rust library.
Protect and verify rights-reservation metadata (with optional steganographic
markers) on PNG, JPEG, and WebP encoded bytes.

> **Status: experimental / local source build.** These bindings are not yet
> published to PyPI. M002 in
> `plans/implementation/language-bindings/002-python-packaging-qualification.md`
> covers wheel qualification and manual release rehearsal.

The Python surface is a thin projection of the canonical Rust byte API
(`process_request_bytes*` and `verify_image_bytes_report`). It does not
reimplement protection logic; the Rust crate is the single semantic source
of truth.

## Encoded bytes are required for metadata

Use the `bytes`-oriented functions (`protect`, `protect_with_report`,
`verify`) on PNG/JPEG/WebP encoded buffers. `DynamicImage`/`PIL.Image`
round-trips through decode/encode will strip container metadata.

## Install (local source build)

```bash
cd bindings/python
python -m venv .venv
. .venv/bin/activate
python -m pip install -U pip maturin pytest
maturin develop --release
```

The wheel installs into the active virtualenv; no Rust toolchain is required
to install a future prebuilt wheel.

## Usage

```python
import stegoeggo

notice = stegoeggo.RightsNotice().with_copyright_holder("Example Corp")
request = (
    stegoeggo.ProtectionRequest.metadata_only(
        notice, stegoeggo.RightsPolicy.PROHIBITED_AI_ML_TRAINING,
    )
    .with_output_format(stegoeggo.ImageOutputFormat.PNG)
    .with_timestamp_override("2026-01-01T00:00:00Z")
    .with_seed(42)
)

with open("input.png", "rb") as f:
    data = f.read()

protected = stegoeggo.protect(data, request)
report = stegoeggo.verify(protected)
print(report.evidence_strength())
```

See `tests/` for full coverage of every entry point.

## License

MIT, same as the underlying Rust crate.
