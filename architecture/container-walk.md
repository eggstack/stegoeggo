# Container Walk

**Source:** `src/container_walk.rs` (~450 lines, `pub(crate)`) — sole bounded container traversal for resource accounting.

Single owner for container work accounting. `observe_container_work()` dispatches on `ImageOutputFormat` to one of three format walkers; all arithmetic uses `checked_add` with lenient break on overflow, and all observations feed `OperationObserver` in `src/resource_limits.rs`.

## Entry point

```rust
pub(crate) fn observe_container_work(
    bytes: &[u8],
    format: ImageOutputFormat,
    budget: &mut OperationObserver,
) -> Result<()>
```

Called once per byte-path execution before heavy work. Never returns a parse error for malformed containers — unknown/truncated input yields `Ok(())` after observing what fits in budget. Strict injection/detection walkers live in their format modules (`protected/metadata_trap/*`, `webp_container.rs`, carrier `jpeg_transcoder/header.rs`) with different error dispositions; do not add hand-written chunk/marker loops in `lib.rs` or `pipeline.rs`.

## Per-format walkers

| Walker | What it walks | What it observes |
|--------|---------------|------------------|
| `observe_png_work` | PNG signature + chunk headers (`len` + `type` + `data` + CRC = `len + 12`), breaks before observing `IEND` (terminator is not counted) and before observing a chunk whose declared data span runs past the buffer | `observe_png_chunk(chunk_total)` per chunk; `observe_metadata_field(data_len)` for `tEXt`/`iTXt` data |
| `observe_jpeg_work` | SOI (`FFD8`), segment markers, stops at EOI (`FFD9`) or SOS (`FFDA`) | `observe_jpeg_segment(seg_len)` per segment; skips stuffing (`FF00`), steps single bytes on non-marker data; `observe_metadata_field(seg_len - 2)` only for markers `0xE1`/`0xED`/`0xFE` |
| `observe_webp_work` | RIFF `WEBP` header + chunk headers (FourCC + LE32 size + padded data) | Per-chunk cost; `observe_metadata_field(chunk_size)` for `XMP `/`EXIF` chunk data; truncated tails observe the partial remainder instead of erroring |

## Invariants

- Bounds-checked: every `pos + N <= len` gate precedes indexing.
- Overflow-safe: `checked_add` everywhere; `None` breaks the loop instead of panicking.
- Truncated spans are not charged: PNG and JPEG both validate the declared segment/chunk span against the buffer length before observing, so malformed input never inflates the budget (the WebP walker is the one exception — it observes the partial remainder of a truncated chunk).
- Lenient: bad magic returns `Ok(())` — accounting never rejects input, enforcement happens in `budget.check_limits()`.
- `IEND`/`EOI`/`SOS` terminate; trailing bytes after `IEND` are not walked.

## Relationship to other modules

- `resource-limits.md` — owns `ResourceLimits` / `OperationObserver` / limit errors; this doc owns the walk.
- `webp-container.md` — strict RIFF parse for injection; the WebP walker here is accounting-only.
- `jpeg-header.md` / `protected-metadata-trap.md` — strict JPEG/PNG walkers for injection and detection.
- Regression coverage: `tests/container_accounting.rs`.
