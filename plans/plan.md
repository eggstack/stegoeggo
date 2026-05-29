# Cloakrs Implementation Plan

## Status: In Progress

This plan consolidates findings from all architecture review files into actionable items.

---

## Wave 1: Critical Bugs (Do First)

### 1.1 `embed_jpeg_stego` Redundancy Bug — Only First Pass Embeds

**File:** `src/protected/steganography.rs:788-841`

**Problem:** When `redundancy > 1`, only the first pass (pass 0) actually embeds payload bits. Subsequent passes (1, 2, etc.) find `embedded >= bits_per_pass` immediately at line 828 and never execute inner embedding logic.

```rust
// steganography.rs:828-830
if embedded >= bits_per_pass {
    break;  // Immediately exits on passes 1,2,3,4
}
```

**Expected:** Each pass should embed the full payload, allowing extraction to use majority voting across redundant embeddings.

**Fix:** After a `break` from the inner loops (embedding complete for a pass), `embedded` must be reset to 0 before the next pass begins, OR the break condition should only exit the inner loops but not the pass-level logic.

**Implementation approach:**
1. After the `if embedded >= bits_per_pass { break; }` chain at lines 828-840, add `embedded = 0;` to reset for next pass
2. OR restructure the loop so `embedded` is reset at the start of each pass iteration
3. Add test with `redundancy=3` verifying all 3 passes embed distinct positions

---

### 1.2 JPEG Segment Truncation — Silent Data Loss

**File:** `src/protected/metadata_trap.rs:676-678`

**Problem:** When parsing a truncated JPEG mid-stream, the code emits partial data and breaks instead of returning an error:

```rust
// metadata_trap.rs:193-196
if pos + 4 > jpeg_data.len() {
    break;
}
output.extend_from_slice(&jpeg_data[pos..]);  // Silent partial output
```

**Impact:** Calling code receives incomplete JPEG data with no error indication.

**Fix:** Return `Error::ImageTruncated` instead of silently truncating:

```rust
if pos + 4 > jpeg_data.len() {
    return Err(Error::ImageTruncated(
        format!("JPEG segment truncation at byte {}", pos)
    ));
}
```

---

### 1.3 Division by Zero Potential in Perturbation

**File:** `src/util/image.rs:560-562`

**Problem:** The `apply_perturbation` function divides by `divisor` with no validation:

```rust
// image.rs:560
output_raw[i] = (px_r + perturbation_offset[0] / divisor).clamp(0, 255) as u8;
```

If `divisor == 0`, this will panic. While callers currently pass safe values, a `debug_assert` would catch programmer errors in debug builds.

**Fix:** Add defensive check:

```rust
// At start of apply_perturbation
debug_assert!(divisor != 0, "divisor must not be zero");
```

Same fix needed for `apply_perturbation_par` at line 616-618.

---

### 1.4 JPEG Segment Length Bounds in Steganography

**File:** `src/protected/steganography.rs` (segment reassembly)

**Problem:** When reassembling JPEG after DCT steganography, malformed segment lengths could cause slice bounds panic:

```rust
// steganography.rs:162-165 (approximate location)
let segment_len = u16::from_be_bytes([jpeg_bytes[pos + 2], jpeg_bytes[pos + 3]]) as usize;
pos += 2 + segment_len;
// Later: &jpeg_bytes[pos..pos + 2 + segment_len] could exceed bounds
```

**Fix:** Add bounds check before slice access:

```rust
if pos + 2 + segment_len > jpeg_bytes.len() {
    return Err(Error::Steganography(
        format!("Malformed JPEG segment length at byte {}", pos)
    ));
}
```

---

## Wave 2: Error Module Improvements

### 2.1 Implement `From<TranscoderError> for Error`

**File:** `src/error.rs`

**Problem:** `TranscoderError` (defined in `jpeg_transcoder/mod.rs`) has no conversion to `Error`. Callers must manually map errors, losing specificity:

```rust
// steganography.rs:126-127 (current pattern)
TranscoderError::HuffmanEncode(...) -> Error::ImageEncode("DCT encode failed: ...")
```

**Fix:** Add implementation to `error.rs`:

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

This enables `?` operator to automatically convert transcoder errors.

---

### 2.2 Remove Dead Error Variants

**File:** `src/error.rs:56`

**Problem:** `Dimensions` and `JpegTranscode` variants are defined but never constructed. Pipeline uses `Error::ImageDecode` for dimension errors. `TranscoderError` is used directly instead of `JpegTranscode`.

**Fix options:**
- **Option A:** Remove `Dimensions` and `JpegTranscode` from the enum (breaking change if public)
- **Option B:** Keep them but add deprecation note

**Recommendation:** Option A — they are dead code and confuse API users. Update any documentation that references them.

---

## Wave 3: CLI Improvements

### 3.1 Extract Batch Duplicate Handling Helper

**File:** `cloakrs-cli/src/main.rs:462-565`

**Problem:** Parallel and serial batch paths contain nearly identical code for detecting and handling duplicate output filenames.

**Fix:** Extract to helper function:

```rust
fn compute_output_path(
    input_path: &Path,
    output_dir: &Option<PathBuf>,
    output_format: &Option<ImageOutputFormat>,
    seen: &mut HashMap<PathBuf, usize>,
) -> Option<PathBuf> {
    // Shared logic from lines 462-494
}
```

---

### 3.2 Improve Error Messages in Batch Mode

**File:** `cloakrs-cli/src/main.rs:596`

**Problem:** Error message only reports count, not which files failed:

```rust
return Err(format!("{} file(s) failed to process", failed_count).into());
```

**Fix:** Collect failed file names and include in error:

```rust
return Err(format!(
    "{} file(s) failed: {}",
    failed_count,
    failed_files.join(", ")
).into());
```

---

## Wave 4: Steganography Improvements

### 4.1 `bits_to_bytes` Release Panic on Non-8-Multiple

**File:** `src/protected/steganography.rs:699-714`

**Problem:** `debug_assert!` only fires in debug builds. In release, `bits.chunks_exact(8)` would panic if input length is not a multiple of 8.

**Fix:** Add runtime check:

```rust
fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    if bits.len() % 8 != 0 {
        return Vec::new();  // Or return error
    }
    // ... existing logic
}
```

**Note:** Currently called from contexts that always pass 208-bit (multiple of 8) chunks, but the function should be defensive.

---

### 4.2 LRU Cache Documentation Fix

**File:** `architecture/protected-precomputed.md`

**Problem:** Docs still warn about "unbounded cache growth" but code uses `LruCache` with bounded capacity (100).

**Fix:** Update docs to reflect LRU eviction behavior. Already identified in stale-items.md but doc may not be updated.

---

## Wave 5: Documentation Improvements

### 5.1 `verify_image_bytes` DCT Stego Behavior

**File:** `architecture/pipeline.md:60`

**Problem:** Docs claim "No DCT stego verification" but code (`steganography.rs:257-288`) does DCT stego verification first, then metadata, then LSB fallback.

**Fix:** Update to reflect actual verification order.

---

### 5.2 JPEG Fast Path Protection Layers

**File:** `architecture/overview.md:63`

**Problem:** Claims "all three protection layers" for Standard/Enhanced/Strong, but JPEG→JPEG path skips perturbation (only applies DCT stego + metadata).

**Fix:** Clarify that JPEG fast path applies only 2 layers (DCT stego + metadata), not 3.

---

## Wave 6: Testing Improvements

### 6.1 Add Test for Redundancy Embedding

**File:** `src/protected/steganography.rs` (test module)

**Add test:**
1. Create test image
2. Embed with `redundancy=3`
3. Extract and verify payload appears 3 times in different coefficient positions
4. Use majority voting to reconstruct payload

### 6.2 Add Test for Error Variant Construction

**File:** `src/error.rs` (test module)

**Add test:** Verify each error variant can be constructed and formatted without panic.

---

## Items from Prior Plan (Completed)

| Task | Status | Notes |
|------|--------|-------|
| Dimension validation in `process_bytes` | ✅ Complete | Validates max_dimension for JPEG and non-JPEG |
| LRU eviction for PrecomputedProtector | ✅ Complete | Uses `lru` 0.12, capacity 100 |
| Seed embedding unit quant error | ✅ Complete | Precondition check for Q-table values < 2 |
| `Option<bool>` documentation | ✅ Complete | Three-state semantics documented |
| CLI batch filename collisions | ✅ Complete | HashMap collision tracking |

---

## Implementation Order

1. **Wave 1** (Critical bugs) — 4 items, should be done in parallel if possible
2. **Wave 2** (Error module) — 2 items, depends on understanding current error handling
3. **Wave 3** (CLI) — 2 items, independent of other waves
4. **Wave 4** (Steganography improvements) — 2 items, independent
5. **Wave 5** (Documentation) — 2 items, can be done anytime
6. **Wave 6** (Testing) — 2 items, after code changes stabilize

**Total: 14 actionable items across 6 waves**

---

## Dependencies

- Wave 1 (critical bugs) must be completed before Wave 4 (stego improvements build on correct embed behavior)
- Wave 2 (error module) should be done early so other waves can use proper error conversions
- Waves 3, 5, 6 are independent and can be done in parallel with other waves

---

## Verification

After each wave, run:
```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```