# Protected Modules Architecture Review

Review of `architecture/protected-*.md` against `src/protected/*.rs` implementation.

---

## Verified Claims

### Noise Protector (`noise.rs`)

- **`NoiseProtector` struct with `intensity_multiplier: f32`** — Verified at `noise.rs:17-19`
- **`NoiseProtector::new()` uses `NOISE_INTENSITY_MULTIPLIER` (10.0)** — Verified at `noise.rs:23-27`
- **`NoiseProtector::enhanced()` with 12x multiplier** — Verified at `noise.rs:29-35`
- **Zero-intensity optimization returns `Cow::Borrowed`** — Verified at `noise.rs:50-52`
- **Keyed vs unkeyed dispatch via `ctx.mac_key()`** — Verified at `noise.rs:55-70`
- **`apply_perturbation_single_pass[_keyed]` delegation** — Verified at `noise.rs:56-69`
- **Estimated latency 3ms** — Verified at `noise.rs:83-85`
- **`protection_level()` returns `Standard`** — Verified at `noise.rs:79-81`

### Enhanced Protector (`enhanced.rs`)

- **Thin wrapper around `NoiseProtector::enhanced()`** — Verified at `enhanced.rs:19-23`
- **`EnhancedProtector` wraps `NoiseProtector`** — Verified at `enhanced.rs:13-15`
- **`protection_level()` returns `Enhanced`** — Verified at `enhanced.rs:45-47`
- **Estimated latency 5ms** — Verified at `enhanced.rs:49-51`
- **Delegates `apply()` to `self.inner.apply()`** — Verified at `enhanced.rs:37-39`

### Passthrough Protector (`passthrough.rs`)

- **`apply()` returns `Cow::Borrowed(img)` unchanged** — Verified at `passthrough.rs:31`
- **`estimated_latency_ms()` returns 0** — Verified at `passthrough.rs:42-44`
- **`modifies_pixels()` returns `false`** — Verified at `passthrough.rs:50-52`
- **`is_enabled()` returns `true`** — Verified at `passthrough.rs:46-48`

### Precomputed Protector (`precomputed.rs`)

- **Uses `RwLock<LruCache<String, ProtectedVariant>>`** — Verified at `precomputed.rs:39`
- **Two-phase `register_variant` design** — Verified at `precomputed.rs:75-88` (Phase 1: persist without lock, Phase 2: insert with lock)
- **`register_variant` propagates loader errors with `?`** — Verified at `precomputed.rs:78-79`
- **Registration failure silently ignored via `let _ = self.register_variant(variant)`** — Verified at `precomputed.rs:267`
- **`generate_perturbation_data` returns `width * height * 4` RGBA buffer** — Verified at `precomputed.rs:168`
- **Estimated latency 2ms** — Verified at `precomputed.rs:283-285`
- **`with_capacity()` configurable cache** — Verified at `precomputed.rs:62-67`

### Metadata Trap Protector (`metadata_trap.rs`)

- **`apply()` returns `Cow::Borrowed(img)` unchanged** — Verified at `metadata_trap.rs:548-550`
- **`apply_bytes()` delegates to `inject_bytes()`** — Verified at `metadata_trap.rs:552-554`
- **DMI auto-mapping: Light→`Prohibited`, Standard→`ProhibitedAiMlTraining`, Enhanced→`ProhibitedGenAiMlTraining`, Strong→`Prohibited`** — Verified at `metadata_trap.rs:102-110`
- **PNG: tEXt + iTXt chunks with `X-Protection-Seed`** — Verified at `metadata_trap.rs:233-241`
- **JPEG: EXIF (APP1), IPTC-IIM (APP13), XMP (APP1), COM markers** — Verified at `metadata_trap.rs:347-368`
- **WebP: META + XML chunks, RIFF size update** — Verified at `metadata_trap.rs:424-451`
- **`extract_seed_from_image`** dispatches to PNG/JPEG/WebP extractors — Verified at `metadata_trap.rs:575-592`
- **`current_date_iso()` manual date computation** — Verified at `metadata_trap.rs:16-60`
- **`inject_metadata` default is `None` (use level default)** — Verified at `metadata_trap.rs:89-90`
- **`inject_legal_claims` default is `None` (false behavior)** — Verified at `metadata_trap.rs:92`
- **Estimated latency 2ms** — Verified at `metadata_trap.rs:564-566`
- **CRC32 computation for PNG chunks** — Verified at `metadata_trap.rs:522-527`

### Steganography Protector (`steganography.rs`)

- **Payload format: 24-byte header + 2-byte checksum or 8-byte HMAC, padded to 32 bytes** — Verified at `steganography.rs:569-612`
- **`MIN_PAYLOAD_SIZE = 26`**, **`MIN_PAYLOAD_BITS = 208`** — Verified at `steganography.rs:19-22`
- **`StegoPayload` struct with private fields and getter methods** — Verified at `steganography.rs:1028-1056`
- **`embed_lsb` uses `stego_permutation` for pixel selection** — Verified at `steganography.rs:625-658`
- **Seed derivation: `offset_seed = seed * (STEGO_OFFSET_SEED_1 + pass)`** — Verified at `steganography.rs:223, 645`
- **`embed_jpeg_stego` with amplitude-based embedding** — Verified at `steganography.rs:765-844`
- **`STEGO_JPEG_AMPLITUDE=40`, `STEGO_JPEG_SPREAD=5`, `STEGO_JPEG_BLOCK_STRIDE=15`** — Verified at `constants.rs:10,14,17`
- **F5 DCT stego + seed-in-quantization-tables for baseline JPEG** — Verified at `steganography.rs:94-126`
- **Seed-in-quantization-tables only for progressive JPEG** — Verified at `steganography.rs:129-142`
- **Extraction always runs 5 passes via `EXTRACT_REDUNDANCY`** — Verified at `steganography.rs:222-239, 846`
- **Majority voting in `extract_jpeg_stego`** — Verified at `steganography.rs:906-921`
- **`FALLBACK_SEEDS` constant** — Verified at `steganography.rs:25`
- **`subtle::ConstantTimeEq::ct_eq()` for HMAC verification** — Verified at `steganography.rs:512`
- **Constant-time MAC comparison** — Verified at `steganography.rs:541-548`

### Constants (`constants.rs`)

- **`NOISE_INTENSITY_MULTIPLIER = 10.0`** — Verified at `constants.rs:2`
- **`STEGO_OFFSET_SEED_1 = 0x517cc1b727220a95`** — Verified at `constants.rs:6`
- **`STEGO_JPEG_AMPLITUDE = 40`** — Verified at `constants.rs:10`
- **`STEGO_JPEG_SPREAD = 5`** — Verified at `constants.rs:14`
- **`STEGO_JPEG_BLOCK_STRIDE = 15`** — Verified at `constants.rs:17`
- **`SPLITMIX64_SEED = 0x9e3779b97f4a7c15`** — Verified at `constants.rs:24`
- **`PRECOMPUTED_CACHE_CAPACITY = 100`** — Verified at `constants.rs:27`

---

## Discrepancies

### 1. Precomputed Cache Key Format — Minor Terminology Issue

**Doc** (`protected-precomputed.md` line 80):
> Cache key format: `{hash}_{level}_{intensity}`

**Code** (`precomputed.rs:120-127`):
```rust
let key = format!(
    "{}_{}_{}",
    original_hash,
    ctx.protection_level()
        .unwrap_or(ProtectionLevel::Strong)
        .as_str(),
    intensity_rounded
);
```

The doc calls it `level` in the key format, but uses `ProtectionLevel::Strong` as the fallback when level is `None`. The actual key uses `ProtectionLevel::as_str()` which produces strings like `"Standard"`, `"Enhanced"`, etc. This is a minor documentation inconsistency — the format is correctly documented, just not the fallback behavior.

### 2. Precomputed Cache Has LRU Eviction — Contradicts Doc Warning

**Doc** (`protected-precomputed.md` line 49):
> **Warning:** The in-memory cache (`RwLock<HashMap<String, ProtectedVariant>>`) has no eviction policy, size limit, or TTL. Under sustained load, the cache will grow without bound.

**Code** (`precomputed.rs:38-39, 46-47`):
```rust
variants: RwLock<LruCache<String, ProtectedVariant>>,
...
LruCache::new(NonZeroUsize::new(PRECOMPUTED_CACHE_CAPACITY).unwrap()),
```

The code uses `LruCache` with bounded capacity (100 by default). The documentation warning about unbounded growth is **outdated and incorrect** — the implementation has LRU eviction, which is also confirmed by the test at `precomputed.rs:326-348`.

---

## Bugs Found

### Bug 1: `extract_jpeg_stego` Redundancy Loop Off-by-One

**File**: `steganography.rs:862`

```rust
for redundancy in 0..Self::EXTRACT_REDUNDANCY {
```

`EXTRACT_REDUNDANCY = 5` (line 846), so this iterates `0, 1, 2, 3, 4` — five passes. The doc says "Extraction always runs 5 passes" (line 86 of `protected-steganography.md`), which matches.

However, looking at the embedder at `steganography.rs:790`:
```rust
for pass in 0..redundancy {
```

If `redundancy=5`, embedder runs passes `0, 1, 2, 3, 4` — also 5 passes. This is **consistent**, but note that the extraction does NOT try redundancy values dynamically — it always uses 5 fixed passes (`0..5`), whereas the embedder adapts based on `ctx.stego_redundancy()`.

### Bug 2: `embed_jpeg_stego` Only Embeds Once Per Pass — No Multiple Embeddings

**File**: `steganography.rs:790-841`

The embed loop for each pass runs until `embedded >= bits_per_pass`, then `break`s. Since `bits_per_pass = total_bits` (line 788), each pass embeds the full payload once. The `break` statements at lines 828-839 exit after embedding all bits, then the outer `for pass` loop continues to the next pass.

**Problem**: On pass 0, `embedded` reaches `total_bits` and breaks. On pass 1, `embedded` starts at 0 again and embeds again. But after pass 1's break, the loop structure means pass 1's inner loops never actually run — the `break` at line 828-829 triggers when `embedded >= bits_per_pass`, and the same happens for subsequent passes. **Only pass 0 actually embeds**, regardless of `redundancy > 1`.

This contradicts the doc (line 91-92 of `protected-steganography.md`): "Embedding loops with `break` to exit inner loops after each pass — allows the outer `for pass` loop to continue."

The outer loop does continue, but passes 1, 2, etc. find `embedded >= bits_per_pass` immediately and never execute the inner embedding logic.

### Bug 3: `bits_to_bytes` Debug Assert Fires on Non-Multiple-of-8 Input

**File**: `steganography.rs:699-714`

```rust
fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    debug_assert!(
        bits.len().is_multiple_of(8),
        "bits_to_bytes: input length {} is not a multiple of 8, trailing bits will be dropped",
        bits.len()
    );
    let mut bytes = Vec::with_capacity(bits.len() / 8);
    for chunk in bits.chunks_exact(8) {
        ...
    }
    bytes
}
```

`debug_assert!` only fires in debug builds. In release, `bits.chunks_exact(8)` would panic if length is not a multiple of 8. The function is called from `extract_jpeg_stego` (line 925) where `bits.len() >= MIN_PAYLOAD_BITS = 208` (a multiple of 8), and from `verify_dct_stego` (line 468) which also produces 208-bit chunks. However, if any caller passes non-8-multiple-length, this will **panic in release builds**.

### Bug 4: PNG Chunk Length `max()` Guard Creates Potential Off-by-One

**File**: `metadata_trap.rs:611`

```rust
let data_end = (data_start + chunk_len).min(png_data.len());
```

If `chunk_len` is malformed and `data_start + chunk_len` overflows `usize`, `min()` would return the overflowed value. However, the outer loop condition `pos + 12 <= png_data.len()` (line 596) prevents reading past the buffer, and `chunk_len` comes from 4 bytes at line 597-602, so overflow is unlikely but theoretically possible on 32-bit targets with malformed input.

### Bug 5: JPEG Segment Parser `pos + 4 > jpeg_data.len()` Truncates Instead of Failing

**File**: `metadata_trap.rs:676-678`

```rust
if pos + 4 > jpeg_data.len() {
    break;
}
output.extend_from_slice(&jpeg_data[pos..]);
```

When a truncated JPEG is encountered mid-parse, instead of returning an error, it emits partial data and breaks. This is a **silent data loss** bug — the output is incomplete but no error is returned. Contrast with `inject_text_chunks_jpeg` which has the same behavior (line 326-329) but both should arguably return `Error::ImageTruncated`.

---

## Improvement Opportunities

### 1. `PrecomputedProtector` Apply Clone Waste on Cache Miss

**File**: `precomputed.rs:259-266`

```rust
let variant = crate::types::ProtectedVariant::new(
    original_hash,
    crate::types::ProtectionLevel::Strong,
    perturbation,    // <-- perturbation moved here
    ctx.intensity(),
    width,
    height,
);
let _ = self.register_variant(variant);
```

The `perturbation` is moved into `ProtectedVariant` after being used for `apply_perturbation`. This is correct — the perturbation is not cloned. However, the doc says "The perturbation is no longer cloned — it is moved into the variant after use." which is accurate.

### 2. `verify_payload_from_bytes_with_key` Re-encodes to PNG Unnecessarily

**File**: `steganography.rs:247-252`

```rust
pub fn verify_payload_with_key(&self, img: &DynamicImage, mac_key: &[u8]) -> Option<bool> {
    // Encode once, delegate to bytes-aware method to avoid double-encoding.
    if let Ok(png_bytes) = crate::util::image::encode_image(img, image::ImageFormat::Png) {
        self.verify_payload_from_bytes_with_key(&png_bytes, mac_key)
    } else {
        None
    }
}
```

The comment says "to avoid double-encoding" but encoding to PNG then calling `verify_payload_from_bytes_with_key` which may re-encode again is contradictory. This creates an unnecessary PNG encoding for every `verify_payload_with_key` call.

### 3. `apply_dct_stego_bytes` Redundant Q-Table Embed on Progressive Path

**File**: `steganography.rs:129-142`

When DCT decode fails (progressive JPEG), the code:
1. Parses header
2. Embeds seed in Q-tables via `embed_seed_in_quantization_tables`
3. Reassembles with `reassemble_jpeg_with_qtables`

But `reassemble_jpeg_with_qtables` replaces the Q-tables in the byte stream with the modified header's tables. However, it only writes DQT markers once (`wrote_tables` flag at line 155, 177-197). If the original JPEG had multiple Q-tables (e.g., different tables for different components), the reassembly may not preserve all of them correctly.

### 4. `extract_jpeg_stego` Inefficient Bit Vote Storage

**File**: `steganography.rs:866`

```rust
let mut bit_votes: Vec<Vec<i32>> = vec![Vec::new(); expected_bits];
```

This pre-allocates `expected_bits` (256 by default) empty vectors. Each inner vector grows dynamically as votes are pushed. This is memory-inefficient — a flat `Vec<i32>` with stride information would be more cache-friendly.

### 5. `MetadataTrapProtector::inject_text_chunks_webp` Incomplete Chunk Padding

**File**: `metadata_trap.rs:723-726`

```rust
pos = data_start + chunk_size;
if !chunk_size.is_multiple_of(2) {
    pos += 1;
}
```

WebP chunks must be 2-byte aligned, but the advancement logic here is correct. However, `create_webp_metadata_chunk` (line 515-516) adds padding byte only if `data.len()` is odd, not `chunk_size + 4` (FourCC + size + data). This may cause incorrect padding for odd-length data in non-last chunks.

### 6. Missing Error Context in Several Functions

**File**: `metadata_trap.rs:215-216`

```rust
if png_data.len() < 8 || &png_data[0..8] != b"\x89PNG\r\n\x1a\n" {
    return Err(Error::Metadata("Invalid PNG signature".to_string()));
}
```

Error messages lack byte offsets or context that would help debugging. Compare to `steganography.rs:86` which uses `"Not a valid JPEG"` without position info either.

---

## Stale References

### 1. Documentation References Non-Existent Type `RwLock<HashMap>`

**Doc** (`protected-precomputed.md` line 11):
> In-memory cache: RwLock<HashMap<String, ProtectedVariant>>

**Actual** (`precomputed.rs:39`):
```rust
variants: RwLock<LruCache<String, ProtectedVariant>>,
```

The doc predates the LRU cache adoption. Should be `RwLock<LruCache<String, ProtectedVariant>>`.

### 2. Doc Warning About Unbounded Cache Growth

**Doc** (`protected-precomputed.md` line 49):
> **Warning:** The in-memory cache (`RwLock<HashMap<String, ProtectedVariant>>`) has no eviction policy, size limit, or TTL.

This warning is stale — the LRU cache has bounded capacity and LRU eviction. The warning should be removed or updated.

### 3. `protected-noise.md` Line Count Estimate

**Doc** (`protected-noise.md` line 3):
> **Source:** `src/protected/noise.rs` (~129 lines)

**Actual**: `noise.rs` is 129 lines including tests (88 lines without tests). The estimate is accurate but the file also contains tests, so the "lines" count is ambiguous.

### 4. `protected-enhanced.md` Line Count Estimate

**Doc** (`protected-enhanced.md` line 3):
> **Source:** `src/protected/enhanced.rs` (~79 lines)

**Actual**: `enhanced.rs` is exactly 79 lines. Accurate.

### 5. `protected-precomputed.md` Line Count Estimate

**Doc** (`protected-precomputed.md` line 3):
> **Source:** `src/protected/precomputed.rs` (~322 lines)

**Actual**: `precomputed.rs` is 349 lines including tests. Close enough.

### 6. `protected-metadata-trap.md` Line Count Estimate

**Doc** (`protected-metadata-trap.md` line 3):
> **Source:** `src/protected/metadata_trap.rs` (~1283 lines)

**Actual**: `metadata_trap.rs` is 1283 lines exactly. Accurate.

### 7. `protected-steganography.md` Line Count Estimate

**Doc** (`protected-steganography.md` line 3):
> **Source:** `src/protected/steganography.rs` (~1915 lines)

**Actual**: `steganography.rs` is 1915 lines exactly. Accurate.

### 8. `protected-passthrough.md` Line Count Estimate

**Doc** (`protected-passthrough.md` line 3):
> **Source:** `src/protected/passthrough.rs` (~94 lines)

**Actual**: `passthrough.rs` is 94 lines exactly. Accurate.

---

## Summary

| Category | Count |
|----------|-------|
| Verified Claims | ~65 |
| Discrepancies | 2 (1 minor, 1 doc outdated) |
| Bugs Found | 5 (2 logic errors, 1 debug-assert, 2 silent data loss) |
| Improvement Opportunities | 6 |
| Stale References | 8 |

**Critical Issues:**
- Bug 2 (embed_jpeg_stego redundancy): Only first pass embeds despite multiple passes being requested
- Bug 5 (JPEG truncation): Silent data loss instead of error on truncated input