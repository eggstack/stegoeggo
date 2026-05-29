# Cloakrs Implementation Plan

## Status: In Progress

This plan consolidates findings from all architecture review files into actionable items.
Verified against source code in `src/` and `cloakrs-cli/src/`.

---

## Parallelization Strategy

Wave 1 splits into 3 parallel tracks (different files, no conflicts):
- **Track A** (`steganography.rs`): Items 1.1, 1.4
- **Track B** (`metadata_trap.rs`): Item 1.2
- **Track C** (`image.rs`): Item 1.3

Wave 2-6 are sequential within each wave but waves can overlap with Wave 1.

---

## Wave 1: Critical Bugs

### 1A.1 `embed_jpeg_stego` Redundancy Bug — Only First Pass Embeds

**File:** `src/protected/steganography.rs:788-841`

**Problem:** When `redundancy > 1`, only the first pass (pass 0) actually embeds payload bits. After pass 0 completes with `embedded >= bits_per_pass`, subsequent passes (1, 2, etc.) hit the `if embedded >= bits_per_pass` check at lines 828/833/837/839 immediately and break without executing inner embedding logic.

**Implementation:**
1. After the inner embedding loops complete (pass 0), `embedded` counter is at `bits_per_pass`
2. Pass 1 starts but `embedded >= bits_per_pass` is already true, so all inner loops break immediately
3. Fix: Add `embedded = 0;` as a new statement after line 840 (after the innermost break chain), before the outer `for pass` loop continues to next iteration

```rust
// After line 840, add reset:
            }
            // FIX: Reset embedded counter for next redundancy pass
            embedded = 0;
        }
```

4. Add test: Create test image, embed with `redundancy=3`, extract and verify payload appears 3 times at different coefficient positions, use majority voting to reconstruct

**Verification:** Run `cargo test --all-features` to ensure new test passes.

---

### 1A.2 JPEG Segment Length Bounds in Steganography Reassembly

**File:** `src/protected/steganography.rs:207-209`

**Problem:** When reassembling JPEG after DCT steganography, a malformed `segment_len` in the input could cause `&jpeg_bytes[pos..pos + 2 + segment_len]` to panic with index out of bounds.

```rust
let segment_len =
    u16::from_be_bytes([jpeg_bytes[pos + 2], jpeg_bytes[pos + 3]]) as usize;
output.extend_from_slice(&jpeg_bytes[pos..pos + 2 + segment_len]);  // No bounds check
pos += 2 + segment_len;
```

**Implementation:**
1. Add bounds check before the slice operation:

```rust
let segment_len =
    u16::from_be_bytes([jpeg_bytes[pos + 2], jpeg_bytes[pos + 3]]) as usize;
if pos + 2 + segment_len > jpeg_bytes.len() {
    return Err(Error::Steganography(format!(
        "Malformed JPEG segment length at byte {}: segment claims {} bytes but only {} remain",
        pos, segment_len, jpeg_bytes.len() - pos
    )));
}
output.extend_from_slice(&jpeg_bytes[pos..pos + 2 + segment_len]);
pos += 2 + segment_len;
```

**Note:** This is the `reassemble_jpeg` function used after DCT stego embedding. Similar patterns in other JPEG parsing code should also be checked.

---

### 1B.1 JPEG Segment Truncation — Silent Data Loss

**File:** `src/protected/metadata_trap.rs:676-678`

**Problem:** When parsing a truncated JPEG mid-stream, the code emits partial data and breaks instead of returning an error:

```rust
if pos + 4 > jpeg_data.len() {
    break;  // Silent partial output
}
output.extend_from_slice(&jpeg_data[pos..]);
```

**Implementation:**
1. Add `ImageTruncated` variant to `src/error.rs`:

```rust
#[error("Image data was truncated: {0}")]
ImageTruncated(String),
```

2. Update the truncation check to return an error:

```rust
if pos + 4 > jpeg_data.len() {
    return Err(Error::ImageTruncated(format!(
        "JPEG segment parsing truncated at byte {} (needed 4 bytes, had {})",
        pos,
        jpeg_data.len() - pos
    )));
}
```

3. Same fix needed for `inject_text_chunks_jpeg` at lines 326-329 (same pattern)

**Note:** `Error::ImageTruncated` does not currently exist in `error.rs` — it must be added first.

---

### 1C.1 Division by Zero Potential in Perturbation

**File:** `src/util/image.rs:560-562` and `image.rs:616-618`

**Problem:** `apply_perturbation` divides by `divisor` with no validation. If `divisor == 0`, this will panic.

```rust
output_raw[i] = (px_r + perturbation_offset[0] / divisor).clamp(0, 255) as u8;
```

**Implementation:**
1. Add `debug_assert!(divisor != 0)` at the start of `apply_perturbation` (after line 538)
2. Add same check to `apply_perturbation_par` (after line 598)

```rust
debug_assert!(divisor != 0, "divisor must not be zero in apply_perturbation");
```

**Note:** Callers pass `divisor` derived from intensity calculations where intensity is clamped [0.0, 1.0], so the divisor should never be zero in practice. The `debug_assert` catches programmer errors in debug builds.

---

## Wave 2: Error Module Improvements

### 2.1 Implement `From<TranscoderError> for Error`

**File:** `src/error.rs`

**Problem:** `TranscoderError` (defined in `jpeg_transcoder/mod.rs:17-35`) has no conversion to `Error`. Callers must manually map errors, losing specificity:

```rust
// Current pattern in steganography.rs:126-127
TranscoderError::HuffmanEncode(...) -> Error::ImageEncode("DCT encode failed: ...")
```

**Implementation:**
1. Add to `src/error.rs`:

```rust
use crate::jpeg_transcoder::TranscoderError;

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

2. After adding this, update callers to use `?` instead of manual mapping. Search for `.map_err(|e| Error::... )` patterns involving TranscoderError.

---

### 2.2 Remove Dead Error Variants

**File:** `src/error.rs:47, 56`

**Problem:** `Dimensions` and `JpegTranscode` variants are defined but never constructed anywhere in the codebase. Pipeline uses `Error::ImageDecode` for dimension errors. `TranscoderError` is used directly instead of `JpegTranscode`.

**Implementation:**
1. **Option A (Recommended):** Remove both variants from the enum (breaking change if public API)
2. **Option B:** Keep them with `#[deprecated]` attributes and documentation noting they are unused

If Option A: Search entire codebase for any external usage that might break. The variants are:
- `Error::Dimensions(String)` — never constructed internally
- `Error::JpegTranscode(String)` — never constructed internally

If removing, update `architecture/error.md` to remove them from the variant list.

---

## Wave 3: Steganography Improvements

### 3.1 `bits_to_bytes` Runtime Check for Non-8-Multiple Input

**File:** `src/protected/steganography.rs:699-714`

**Problem:** `debug_assert!` only fires in debug builds. In release, `bits.chunks_exact(8)` would panic if input length is not a multiple of 8.

```rust
fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    debug_assert!(
        bits.len().is_multiple_of(8),
        ...
    );
    // ...
    for chunk in bits.chunks_exact(8) {  // Panics in release if not multiple of 8
```

**Implementation:**
1. Add runtime validation:

```rust
fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    if bits.len() % 8 != 0 {
        return Vec::new();  // Defensive: return empty for invalid input
    }
    // ... existing logic
}
```

**Note:** Currently called from contexts that always pass 208-bit (multiple of 8) chunks, but the function should be defensive. Return empty Vec rather than error to avoid changing call sites.

---

## Wave 4: CLI Improvements

### 4.1 Extract Batch Duplicate Handling Helper

**File:** `cloakrs-cli/src/main.rs:462-565`

**Problem:** Parallel and serial batch paths contain nearly identical code for detecting and handling duplicate output filenames.

**Implementation:**
Extract to helper function:

```rust
fn compute_output_path(
    input_path: &Path,
    output_dir: &Option<PathBuf>,
    output_format: &Option<ImageOutputFormat>,
    seen: &mut HashMap<PathBuf, usize>,
) -> Option<PathBuf> {
    // Shared logic from lines 462-494:
    // 1. Compute stem from input_path
    // 2. Determine extension (explicit format or detect from magic bytes)
    // 3. Check/update seen HashMap for collision tracking
    // 4. Handle collision suffix (_protected_1, _protected_2, etc.)
    // Return Some(PathBuf) for success, None for skip
}
```

Both the parallel path (`par_iter` at ~462-515) and serial path (`iter` at ~517-565) should call this helper.

---

### 4.2 Improve Error Messages in Batch Mode

**File:** `cloakrs-cli/src/main.rs:596`

**Problem:** Error message only reports count, not which files failed:

```rust
return Err(format!("{} file(s) failed to process", failed_count).into());
```

**Implementation:**
1. Change `failed_count: usize` to `failed_files: Vec<PathBuf>` to track failed file paths
2. Update error message:

```rust
return Err(format!(
    "{} file(s) failed to process: {}",
    failed_files.len(),
    failed_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
).into());
```

3. Update the error collection point to push to the Vec instead of incrementing a counter

---

## Wave 5: Testing Improvements

### 5.1 Add Test for Redundancy Embedding

**File:** `src/protected/steganography.rs` (test module)

**Test approach:**
1. Create small test image (8x8 grayscale minimum for DCT coefficients)
2. Generate a known payload (e.g., 256 bits)
3. Embed with `redundancy=3` using known seed
4. Extract with `EXTRACT_REDUNDANCY=5` (default extraction uses 5 passes)
5. Verify extracted payload matches embedded payload (majority voting works)
6. Verify that extraction uses majority voting across redundant embeddings

**Note:** The `embed_jpeg_stego` bug (1A.1 above) must be fixed first, otherwise this test will fail because only the first pass embeds.

---

### 5.2 Add Test for Error Variant Construction

**File:** `src/error.rs` (test module)

**Test approach:**
1. Construct each error variant with a string message
2. Verify `std::fmt::Display` produces expected output (no panic)
3. Verify the error can be converted to string via `to_string()`
4. Optionally: verify `From` implementations work for `std::io::Error` and `serde_json::Error`

---

## Items from Prior Plan (Completed)

| Task | Status | Notes |
|------|--------|-------|
| Dimension validation in `process_bytes` | ✅ Complete | `validate_jpeg_dimensions_from_bytes()` + validation on non-JPEG path |
| LRU eviction for PrecomputedProtector | ✅ Complete | Uses `lru` 0.12, capacity 100, LRU eviction on insert |
| Seed embedding unit quant error | ✅ Complete | Precondition check for Q-table values < 2 |
| `Option<bool>` documentation | ✅ Complete | Three-state semantics documented |
| CLI batch filename collisions | ✅ Complete | HashMap collision tracking |
| LRU Cache documentation | ✅ Complete | `LruCache` with bounded capacity documented |
| Parallel threshold documentation | ✅ Complete | 1 core: 4096, 4 cores: 16384, 16 cores: 65536 |
| verify_image_bytes DCT stego | ✅ Complete | pipeline.md correctly describes DCT stego verification |

---

## Implementation Order

**Phase 1 (Parallel - 3 tracks):**
- Track A: Items 1A.1 + 1A.2 (steganography.rs)
- Track B: Item 1B.1 (metadata_trap.rs + error.rs for ImageTruncated)
- Track C: Item 1C.1 (image.rs)

**Phase 2 (After Phase 1):**
- Wave 2: Items 2.1 + 2.2 (error.rs)

**Phase 3 (Independent):**
- Wave 3: Item 3.1 (steganography.rs)
- Wave 4: Items 4.1 + 4.2 (cli/main.rs)
- Wave 5: Items 5.1 + 5.2 (testing)

**Total: 11 actionable items across 5 waves**

---

## Dependencies

- Wave 1 (critical bugs) must be completed before Wave 3 (stego improvements) since 3.1 builds on correct embed behavior
- Wave 2 (error module) should be done early so other waves can use proper error conversions
- Wave 4 (CLI) is independent and can be done anytime
- Wave 5 (testing) should be done after code changes stabilize

---

## Verification

After each wave, run:
```bash
cargo test --all-features
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

For steganography changes specifically:
```bash
cargo test steganography
cargo test test_embed_jpeg_stego  # After implementing 5.1
```

---

## File Reference Quick Guide

| File | Items |
|------|-------|
| `src/protected/steganography.rs` | 1A.1, 1A.2, 3.1, 5.1 |
| `src/protected/metadata_trap.rs` | 1B.1 |
| `src/util/image.rs` | 1C.1 |
| `src/error.rs` | 1B.1 (add variant), 2.1, 2.2, 5.2 |
| `cloakrs-cli/src/main.rs` | 4.1, 4.2 |
