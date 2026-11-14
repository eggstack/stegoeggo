# stegoeggo Python binding

Python interface to the `stegoeggo` Rust library for protecting and verifying
rights metadata on PNG, JPEG, and WebP encoded bytes.

**Status:** experimental and not published to PyPI. Use a source checkout to
build it locally. The binding targets CPython 3.11 through 3.14.

## Install from source

From the repository root:

```bash
cd bindings/python
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -U pip maturin
maturin develop --release
```

A local build requires Rust 1.89 or later. Prebuilt wheels are not currently
distributed through PyPI.

## Usage

Use encoded image bytes when rights metadata must remain in the output file.
Decoding to pixels and encoding again can discard container metadata.

```python
import stegoeggo

notice = stegoeggo.RightsNotice().with_copyright_holder("Example Corp")
request = stegoeggo.ProtectionRequest.metadata_only(
    notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
).with_seed(42).with_timestamp_override("2026-01-01T00:00:00Z")

with open("input.png", "rb") as source:
    data = source.read()

protected = stegoeggo.protect(data, request)
report = stegoeggo.verify(protected)
print(report.evidence_strength)
```

`protect_file` and `verify_file` are convenience wrappers. `protect_file` writes
the completed output through a same-directory temporary file before replacing
the destination, so a failed operation does not leave a partial image.

## Errors and keys

Protection and verification errors expose structured fields for capacity and
resource-limit failures. Branch on the exception type or documented attributes,
not on the exact wording of a message. Secret MAC key bytes are not included in
exception messages or attributes.

## License

MIT, same as the Rust library.
