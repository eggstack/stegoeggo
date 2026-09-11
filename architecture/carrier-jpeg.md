# Carrier JPEG (DCT Domain)

**Sources:** `stegoeggo-stego/src/jpeg.rs` (~2100 lines, public carrier) · `prepared.rs` (~426 lines, public reuse handle). Internals: `jpeg-header.md`, `jpeg-entropy.md`, `jpeg-stego-f5.md`, `jpeg-transcoder.md`.

Encoded-JPEG DCT carrier with container-preserving encode. Operates on baseline sequential DCT coefficients; progressive JPEGs are probed as unsupported and take the seed-hint path only.

## Public API (`jpeg::`)

| Group | Functions |
|-------|-----------|
| Probe | `inspect(bytes, min_dim, max_dim)`, `probe_support(bytes) -> JpegSupport::{Supported, Unsupported(reason)}`, `is_progressive_jpeg(bytes)` |
| Capacity | `capacity(bytes, payload_len, &JpegConfig) -> CapacityReport` — units are eligible AC coefficients with `\|coef\| >= 2` after canonicalization |
| Raw | `embed` (best-effort: auto-downgrade redundancy, seed-only fallback) / `extract(bytes, len, &config, actual_redundancy)` — pass `report.actual_redundancy` to `extract` |
| Strict | `embed_strict` / `embed_framed_strict` — exact requested redundancy or `InsufficientCapacity`, no output |
| Framed | `embed_framed` / `extract_framed` — `frame::{encode,decode}` wrapper; length self-describing |
| Tiled | `embed_tiled` / `extract_tiled` / `embed_tiled_framed` / `extract_tiled_framed` — redundancy 1 per tile; tile size must be ≥ 8 and a multiple of 8 |
| Seed hint | `embed_seed_hint(bytes, seed)` (transactional: full 96-bit hint or `InsufficientCapacity`) / `extract_seed_hint(bytes) -> Option<u64>` |

`JpegConfig { seed, redundancy }` mirrors `LsbConfig`: `from_redundancy` / `with_redundancy_value` for runtime values (default redundancy 3); `with_redundancy` is constants-only. Zero seeds valid.

## Best-effort vs strict

Best-effort `embed` is the parent rights crate's application policy (downgrade, then seed-only fallback), not generic carrier semantics. Generic callers that need exactness use `*_strict`, which embed at exactly the requested redundancy in one pass — no retry loop — computing max feasible redundancy from capacity first, then embedding and encoding once.

## Prepared reuse (`prepared::`)

`PreparedJpeg<'a>` borrows the encoded bytes and retains one coefficient decode across repeated operations:

```rust
let p = PreparedJpeg::new(&jpeg_bytes)?;
p.support(); p.capacity(len, &cfg); p.extract(len, &cfg, r);
p.extract_framed(&cfg); p.extract_tiled(len, &tile, n);
p.embed_strict(payload, &cfg); p.embed_framed_strict(payload, &cfg); p.seed_hint();
```

`Debug + Send + Sync`. Single-decode-per-operation also holds without `PreparedJpeg`: `extract_framed` retains private decoded coefficients for its bounded redundancy search; the parent's hidden `JpegSearchContext` shares one decode across standard probing and tiled fallback. Do not recompose either from public `capacity`/`extract` calls or add per-redundancy extract calls.

## Container preservation

Re-encode replaces only DQT and SOS scan data; APP/COM/unknown segments survive verbatim. Q-table hint capacity counts eligible positions (values ≥ 2) across the first two tables before mutating. JPEG→JPEG protection uses this byte-only fast path (no pixel decode); JPEG→PNG/WebP is one pixel decode plus raster LSB, never transient DCT.

## Errors

`StegoError::{InsufficientCapacity, UnsupportedJpeg(JpegUnsupportedReason), ...}` — 10 `JpegUnsupportedReason` variants (progressive, multi-scan, etc.). See `carrier-surface.md` for the shared report/error types.
