# stegoeggo Python bindings

Python frontend for the [`stegoeggo`](https://docs.rs/stegoeggo) Rust library.
Protect and verify rights-reservation metadata (with optional steganographic
markers) on PNG, JPEG, and WebP encoded bytes.

> **Status: experimental / local source build.** These bindings are not yet
> published to PyPI. M003 in
> `plans/implementation/language-bindings/003-python-corrective-qualification.md`
> is the corrective qualification milestone; multi-platform wheels and the
> direct sdist install path are produced by the manually-dispatched
> `.github/workflows/release-python.yml`.

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

## Install (prebuilt wheel or sdist)

When the manually-dispatched `.github/workflows/release-python.yml`
artifacts are attached to a GitHub Release, install them with a normal
`pip install <artifact>` in any CPython ≥ 3.11 environment. The sdist is a
single-source-build artefact that does not require `maturin` to be present
on the install machine:

```bash
python -m pip install stegoeggo-0.4.2.tar.gz
```

## Usage

```python
import stegoeggo

notice = stegoeggo.RightsNotice().with_copyright_holder("Example Corp")
request = (
    stegoeggo.ProtectionRequest.metadata_only(
        notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
    )
    .with_output_format(stegoeggo.ImageOutputFormat.Png)
    .with_timestamp_override("2026-01-01T00:00:00Z")
    .with_seed(42)
)

with open("input.png", "rb") as f:
    data = f.read()

protected = stegoeggo.protect(data, request)
report = stegoeggo.verify(protected)
print(report.evidence_strength)
```

## File helpers

`protect_file` and `verify_file` are pure-Python wrappers over the byte
API. They are failure-safe: `protect_file` reads the source, runs the
canonical protect path, and only replaces the destination after the full
output has been written to a same-directory temporary file and `fsync`ed.
On any failure the original file is untouched and any leftover temporary
file is best-effort cleaned up.

```python
import stegoeggo

notice = stegoeggo.RightsNotice().with_copyright_holder("Example Corp")
request = stegoeggo.ProtectionRequest.metadata_only(
    notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
).with_seed(42).with_timestamp_override("2026-01-01T00:00:00Z")

# In-place replacement (unchanged two-argument form)
stegoeggo.protect_file("input.png", request)

# Explicit output path: never mutates the input file
stegoeggo.protect_file("input.png", request, "input.protected.png")

report = stegoeggo.verify_file("input.protected.png")
```

## Structured exceptions

The exception classes raised by `protect*` / `verify` carry the same
structured fields as the canonical Rust `Error` variants. `InsufficientCapacityError`
exposes `required` and `available` carrier-unit counts. `ResourceLimitError`
exposes `resource` together with the structured `size`/`limit`, `kind`/`count`/`limit`,
or `width`/`height`/`max_width`/`max_height` triple that triggered it. The
carrier-level `resource="carrier"` case preserves the canonical message with
no numeric fields. `kind` is descriptive Rust-provided context, so branch on
`resource` or the exception class rather than exact `kind` wording. Secret
MAC/HMAC key bytes never appear in exception messages or attributes.

```python
try:
    stegoeggo.protect(payload, request)
except stegoeggo.InsufficientCapacityError as e:
    print(f"need {e.required} units, have {e.available}")
except stegoeggo.ResourceLimitError as e:
    print(f"resource={e.resource} kind={e.kind} count={e.count}/{e.limit}")
```

See `tests/` for full coverage of every entry point.

## License

MIT, same as the underlying Rust crate.

