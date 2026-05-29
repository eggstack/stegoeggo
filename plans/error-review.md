# Error Module Review

**Reviewed:** `architecture/error.md` vs `src/error.rs`
**Date:** 2026-05-29

---

## Verified Claims

| Claim | Status | Notes |
|-------|--------|-------|
| Source file `src/error.rs` (~69 lines) | ✅ | Actual: 69 lines |
| Uses `thiserror` | ✅ | `use thiserror::Error` at line 8 |
| `#[non_exhaustive]` attribute | ✅ | Present at line 11 |
| All 13 error variants documented | ✅ | Enum matches doc |
| `pub type Result<T> = std::result::Result<T, Error>` | ✅ | Line 69 |
| `#[cfg(feature = "async")]` on `Task` | ✅ | Line 64 |
| `#[from]` on `Io` and `Serialization` | ✅ | Lines 20, 23 |
| `#[from]` on `Image` variant | ✅ | Line 41 |
| Error variant descriptions | ✅ | All match source |

---

## Discrepancies

### 1. `Dimensions` variant is dead code
- **Doc says:** `Dimensions` — Pipeline — Image dimensions exceed limits
- **Actual:** `Dimensions` (line 56) is defined but **never constructed** anywhere in the codebase
- **Pipeline uses:** `Error::ImageDecode` for dimension errors (e.g., `lib.rs:183`, `lib.rs:351`)
- **Impact:** Low — dead code doesn't cause runtime issues but clutters the API

### 2. `JpegTranscode` variant is dead code
- **Doc says:** `JpegTranscode` — `jpeg_transcoder` — JPEG coefficient decode/encode failures
- **Actual:** `JpegTranscode` (line 47) is defined but **never constructed** anywhere
- **Actual error type used:** `TranscoderError` (defined in `jpeg_transcoder/mod.rs:17-35`) is a separate enum
- **Code using transcoder errors:** All `TranscoderError` variants are constructed directly and never converted to `Error::JpegTranscode`
- **Example:** `stego_f5.rs:88` returns `TranscoderError::EmbeddingFailed`, not `Error::JpegTranscode`
- **Impact:** Low — dead code clutters the public API; users might expect this variant to be used

### 3. `Image` variant doc is misleading
- **Doc says:** `Image` — General — Image processing errors (dimensions, format, etc.)
- **Actual doc comment (line 37-39):** States this variant wraps `image::ImageError` directly and is for "image crate internals without additional wrapping"
- **Actual usage:** Used for pixel decode/encode errors in `image` crate internals
- **Impact:** Doc treats it as a general error; code comment clarifies it's for internal crate errors only

---

## Bugs Found

### 1. No `From` implementation bridging `TranscoderError` → `Error`
- **Location:** No implementation exists; `steganography.rs:126-127` manually maps with `map_err`
- **Issue:** When `JpegTranscoder::encode_coefficients` returns `TranscoderError`, the caller must manually format it into `Error::ImageEncode` or `Error::Steganography`. This loses error semantics (e.g., `TranscoderError::HuffmanEncode` becomes just a string in `Error::ImageEncode`).
- **Example:** `steganography.rs:127` — `TranscoderError::HuffmanEncode(...)` becomes `Error::ImageEncode("DCT encode failed: ...")` — loss of `HuffmanEncode` variant information
- **Impact:** Errors bubbled up from the transcoder lose their specific variant, making debugging harder

### 2. `Dimensions` variant defined but never used
- **Location:** `error.rs:56`
- **Issue:** The variant is purely dead code. If a user catches `Error::Dimensions`, it can never be triggered by the library.
- **Impact:** Confusion for API users who see `Dimensions` and expect it to be used

---

## Improvement Opportunities

### 1. Consider removing `Dimensions` from `Error` enum
- **Rationale:** Currently unused; documented but never constructed
- **Alternative:** Convert callers in `lib.rs` to use `Dimensions` instead of `ImageDecode` if `Dimensions` semantics are desired

### 2. Implement `From<TranscoderError> for Error`
```rust
impl From<TranscoderError> for Error {
    fn from(e: TranscoderError) -> Self {
        match e {
            TranscoderError::InvalidFormat(s) => Error::InvalidFormat(s),
            TranscoderError::Unsupported(s) => Error::Image(s),
            TranscoderError::HuffmanDecode(s) => Error::JpegTranscode(s),
            TranscoderError::HuffmanEncode(s) => Error::JpegTranscode(s),
            TranscoderError::Io(e) => Error::Io(e),
            TranscoderError::EmbeddingFailed(s) => Error::Steganography(s),
        }
    }
}
```
- **Rationale:** Would automatically convert transcoder errors at `?` operators, preserving error semantics through the call stack

### 3. Consider adding `#[non_exhaustive]` to `TranscoderError` in `jpeg_transcoder/mod.rs`
- **Rationale:** Matches the pattern used by `Error` enum; prevents downstream exhaustive matching

### 4. Add test coverage for error variant construction
- **Rationale:** No tests verify that each error variant can be constructed and formatted correctly; prevents accidental breakage

---

## Stale References

None found — the documented variant names match the actual enum exactly.

---

## Summary

The error module is well-implemented with correct `#[from]` derive macros enabling ergonomic error propagation. The main issues are:

1. **`Dimensions` and `JpegTranscode` are dead code** — never constructed but publicly exported
2. **No bridge from `TranscoderError` to `Error`** — transcoder errors lose specificity when propagated
3. **Doc for `Image` variant understates its purpose** — code comment clarifies it's for image crate internals, not general use
