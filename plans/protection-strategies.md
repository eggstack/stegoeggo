# Protection Strategies Review Findings

## Document: constants.md

### Verified Claims

- `NOISE_INTENSITY_MULTIPLIER` = 10.0 — **Confirmed** (`constants.rs:2`)
- `STEGO_OFFSET_SEED_1` = 0x517cc1b727220a95 — **Confirmed** (`constants.rs:6`)
- `STEGO_JPEG_AMPLITUDE` = 40 — **Confirmed** (`constants.rs:10`)
- `STEGO_JPEG_SPREAD` = 5 — **Confirmed** (`constants.rs:14`)
- `STEGO_JPEG_BLOCK_STRIDE` = 15 — **Confirmed** (`constants.rs:17`)
- `XORSHIFT_SEED_OFFSET` = 0x123456789ABCDEF0 — **Confirmed** (`constants.rs:21`)
- `SPLITMIX64_SEED` = 0x9e3779b97f4a7c15 — **Confirmed** (`constants.rs:24`)
- Module interactions (noise.rs, steganography.rs, util/image.rs, util/seed.rs) — **Confirmed** via imports

### Discrepancies

- None.

### Improvement Opportunities

- None.

### Potential Bugs/Edge Cases

- None.

---

## Document: protected-passthrough.md

### Verified Claims

- `apply()` returns `Cow::Borrowed(img)` — **Confirmed** (`passthrough.rs:31`)
- `name()` returns `"passthrough"` — **Confirmed** (`passthrough.rs:34`)
- `protection_level()` returns `ProtectionLevel::Disabled` — **Confirmed** (`passthrough.rs:38`)
- `estimated_latency_ms()` returns 0 — **Confirmed** (`passthrough.rs:42`), but see Discrepancies below
- `modifies_pixels()` returns `false` — **Confirmed** (`passthrough.rs:50`)
- Zero allocation, zero copy behavior — **Confirmed**

### Discrepancies

1. **`is_enabled()` returns `false` — FALSE.** The document claims `is_enabled()` returns `false` and that "the pipeline skips this protector entirely." In fact, `passthrough.rs:46` returns `true`. The pipeline does NOT use `is_enabled()` for routing — it does a direct `match level` dispatch in `lib.rs:223-235`. The `is_enabled()` method is defined in the `Protector` trait with a default of `true`, but it is never called by the pipeline. This is a documentation error; the claim about `is_enabled()` returning `false` is incorrect.

2. **`estimated_latency_ms()` return type.** The document shows the return type as `f64` in the code block, but the actual trait signature is `fn estimated_latency_ms(&self) -> u32` (`traits.rs:88`). The implementation returns `0` (integer), not `0.0` (float).

### Improvement Opportunities

- The document should either remove the `is_enabled()` claim or correct it to return `true`.

### Potential Bugs/Edge Cases

- The `is_enabled()` method is currently unused by the pipeline. If it were used for skip optimization, returning `true` for a no-op protector would be a missed optimization.

---

## Document: protected-noise.md

### Verified Claims

- `NoiseProtector` has `intensity_multiplier: f32` field — **Confirmed** (`noise.rs:18`)
- `NoiseProtector::new()` uses `NOISE_INTENSITY_MULTIPLIER` (10.0) — **Confirmed** (`noise.rs:23-26`)
- `NoiseProtector::enhanced()` uses 12.0 — **Confirmed** (`noise.rs:31-35`)
- Intensity scaling: raw intensity multiplied by multiplier before noise amplitude — **Confirmed** (delegates to `apply_perturbation_single_pass` which takes the multiplier)
- Zero intensity optimization: returns `Cow::Borrowed(img)` when `intensity <= 0.0` — **Confirmed** (`noise.rs:50-52`)
- Keyed path uses `apply_perturbation_single_pass_keyed` — **Confirmed** (`noise.rs:55-62`)
- Unkeyed path uses `apply_perturbation_single_pass` — **Confirmed** (`noise.rs:64-69`)
- `protection_level()` returns `ProtectionLevel::Standard` — **Confirmed** (`noise.rs:79-81`)

### Discrepancies

1. **`estimated_latency_ms()` return type.** Document shows `0.0` (f64), actual return type is `u32` returning `3`.

2. **`NoiseProtector` does NOT wrap `NoiseProtector::enhanced()` internally.** The document describes `NoiseProtector` as accepting the intensity multiplier in its field, which is correct. However, the document header code block for `NoiseProtector` incorrectly labels the field documentation as `10.0 for Standard, 12.0 for Enhanced` — the `NoiseProtector` type itself doesn't know which level it serves; that's determined by which constructor is called.

### Improvement Opportunities

- None.

### Potential Bugs/Edge Cases

- None.

---

## Document: protected-enhanced.md

### Verified Claims

- `EnhancedProtector` wraps `NoiseProtector` via `inner` field — **Confirmed** (`enhanced.rs:14`)
- Uses `NoiseProtector::enhanced()` with 12x multiplier — **Confirmed** (`enhanced.rs:21`)
- Delegates `apply()` to `self.inner.apply()` — **Confirmed** (`enhanced.rs:38`)
- `protection_level()` returns `ProtectionLevel::Enhanced` — **Confirmed** (`enhanced.rs:45-47`)
- Exists as separate type for pipeline routing and different protection level — **Confirmed**

### Discrepancies

1. **`estimated_latency_ms()` values.** Document claims ~7.0 for Enhanced vs ~5.0 for Standard. Actual code: Enhanced returns `5` (`enhanced.rs:50`), Standard (NoiseProtector) returns `3` (`noise.rs:84`). The ratios and absolute values are both different from documentation. Additionally, the return type is `u32`, not `f64`.

### Improvement Opportunities

- None.

### Potential Bugs/Edge Cases

- None.

---

## Document: protected-metadata-trap.md

### Verified Claims

- `apply()` returns `Cow::Borrowed(img)` unchanged — **Confirmed** (`metadata_trap.rs:548-549`)
- `apply_bytes()` delegates to `inject_bytes()` — **Confirmed** (`metadata_trap.rs:552-554`)
- Pipeline routes `Light` level through `apply_light_bytes()` — **Confirmed** (`lib.rs:225`, `lib.rs:290-303`)
- DMI auto-mapping: Light→Prohibited, Standard→ProhibitedAiMlTraining, Enhanced→ProhibitedGenAiMlTraining, Strong→Prohibited — **Confirmed** (`metadata_trap.rs:103-108`)
- All seven DmiValue variants — **Confirmed** (`metadata_trap.rs:102-109`)
- `name()` returns `"metadata_trap"` — **Confirmed** (`metadata_trap.rs:557`)
- `protection_level()` returns `ProtectionLevel::Light` — **Confirmed** (`metadata_trap.rs:560-562`)
- Seed extraction from PNG tEXt, JPEG COM, WebP META — **Confirmed** (`metadata_trap.rs:575-592`)
- `current_date_iso()` manual computation (no chrono) — **Confirmed** (`metadata_trap.rs:16-59`)
- Legal metadata injection (copyright, contact, license, usage terms, date) — **Confirmed** (`metadata_trap.rs:126-159`)
- Format-specific paths: PNG (tEXt, iTXt/XMP), JPEG (EXIF APP1, IPTC-IIM APP13, XMP APP1, COM), WebP (XML, META) — **Confirmed**

### Discrepancies

1. **Metadata injection does NOT survive encode/decode cycles.** The document correctly notes that `apply()` returns `Cow::Borrowed` unchanged because metadata cannot survive through the `DynamicImage` API. This is accurate. However, the document's statement "Injects four marker types: 1. EXIF (APP1), 2. IPTC-IIM (APP13), 3. XMP (APP1), 4. COM" for JPEG is slightly misleading — these markers are injected as raw bytes into the byte stream, not via the `DynamicImage` API. The metadata does survive if the bytes are preserved (e.g., `apply_bytes()` path), but is lost if the image is decoded to pixels and re-encoded.

2. **DMI auto-mapping: Strong→Prohibited.** The document says Strong maps to `Prohibited`. The code (`metadata_trap.rs:107`) confirms this. However, this is semantically odd — Strong is the highest protection level but maps to the same DMI as Light (Prohibited). The doc does not comment on this design choice.

### Improvement Opportunities

- The document could clarify that DMI auto-mapping for Strong uses `Prohibited` (same as Light), not a stronger variant like `ProhibitedAnyProcessing`.

### Potential Bugs/Edge Cases

- `apply_light_bytes()` in `lib.rs:302` decodes the metadata-injected bytes back to `DynamicImage` via `image::load_from_memory`. This means the metadata (tEXt/iTXt/COM/XMP markers) survives in the byte output because it was injected AFTER encoding, but when the caller receives a `DynamicImage` from `process()`, the metadata is already embedded in the byte representation. The next call to `load_from_memory` by a consumer would parse and preserve the metadata. This is the correct design.

---

## Document: protected-precomputed.md

### Verified Claims

- `PrecomputedProtector` uses `RwLock<HashMap<String, ProtectedVariant>>` — **Confirmed** (`precomputed.rs:37`)
- Optional `Box<dyn VariantLoader>` — **Confirmed** (`precomputed.rs:38`)
- Two-phase registration (persist without lock, then insert with write lock) — **Confirmed** (`precomputed.rs:65-78`)
- Cache miss: generate, auto-register (best-effort), apply — **Confirmed** (`precomputed.rs:244-268`)
- `generate_perturbation_data` creates RGBA buffer — **Confirmed** (`precomputed.rs:157-187`)
- `register_variants` batch registration — **Confirmed** (`precomputed.rs:85-106`)
- Registration failure silently ignored (best-effort caching) — **Confirmed** (`precomputed.rs:264-267`)
- `apply()` returns `Cow::Borrowed` when intensity is 0.0 — **Not implemented.** The `apply()` method (`precomputed.rs:232-270`) does NOT have a zero-intensity early return. It always computes the hash, checks cache, and generates/applies perturbation. This is a discrepancy from the document.

### Discrepancies

1. **`estimated_latency_ms()` return type.** Document code block shows `0.0` (f64), actual return type is `u32` returning `2`.

2. **Cache key format.** Document claims `{uuid}_{hash}_{intensity}`. Actual code (`precomputed.rs:114-121` and `types.rs:600-604`) uses `{hash}_{level}_{intensity}`. There is no `uuid` in the cache key.

3. **ProtectedVariant fields.** Document lists `uuid` as a field of `ProtectedVariant`. The `ProtectedVariant` struct in `types.rs` does contain `uuid` (for identification), but the cache key does NOT include it. The `uuid` is an identifier, not part of the lookup key.

4. **Zero-intensity early return missing.** Document claims `apply()` returns `Cow::Borrowed` when intensity is 0.0. The actual `apply()` method does not check for zero intensity. It will always generate/cache perturbation data, even when intensity is 0.0 (the perturbation will just be all zeros around 128.0).

5. **`apply()` does not use `is_enabled()`.** No mention of this in the document, but consistent with other protectors — the pipeline does not check `is_enabled()`.

### Improvement Opportunities

- None.

### Potential Bugs/Edge Cases

- **Unbounded cache growth.** The `RwLock<HashMap>` has no eviction policy, size limit, or TTL. Under sustained load with varying images, the cache will grow without bound until memory exhaustion. The document does not mention this limitation.

- **Read lock during apply (cache hit path).** On cache hit (`precomputed.rs:124-132`), a read lock is held while cloning the variant. `ProtectedVariant` contains `Vec<u8>` for perturbation data, so cloning involves heap allocation. If variants are large (e.g., high-resolution images), this clone could be slow under contention. The document correctly identifies this as a potential thread-safety concern.

- **Write lock during cache miss fallback.** On loader cache miss (`precomputed.rs:136-143`), a write lock is acquired to populate the in-memory cache. If the loader I/O is slow, this blocks all readers and writers. The two-phase design mitigates this for `register_variant`, but not for the `get_cached_variant` loader fallback path.

---

## Document: protected-steganography.md

### Verified Claims

- Payload format structure (version, level, seed, intensity, timestamp, reserved, HMAC/checksum) — **Partially confirmed.** The actual layout is: version (1 byte), protection level (1 byte), seed (8 bytes LE), intensity (2 bytes LE, scaled by 100), timestamp (8 bytes LE), then 2 bytes checksum or 8 bytes HMAC. The "Reserved (zeroed)" field at offset 20-23 is NOT explicitly reserved — the timestamp occupies bytes 12-19, and bytes 20-23 happen to be zero because the timestamp is typically small. The code pads to 24 bytes with zeros (`steganography.rs:589-593`), but this is just zero-padding, not a named "Reserved" field.

- `MIN_PAYLOAD_SIZE` = 32 — **Incorrect.** Code defines `MIN_PAYLOAD_SIZE = 26` (`steganography.rs:20`). The payload is padded to 32 bytes by `generate_payload`, but `MIN_PAYLOAD_SIZE` is 26 (the minimum valid payload size for parsing: 24 header + 2 checksum).

- `MIN_PAYLOAD_BITS` = 256 — **Incorrect.** Code defines `MIN_PAYLOAD_BITS = MIN_PAYLOAD_SIZE * 8 = 208` (`steganography.rs:22`).

- `StegoPayload` has private fields with getters — **Confirmed** (`steganography.rs:1029-1056`)

- LSB embedding with collision-free LCG permutation — **Confirmed** (`steganography.rs:619-623`)

- Seed derivation: `offset_seed = seed * (STEGO_OFFSET_SEED_1 + pass)` — **Confirmed** (`steganography.rs:223`, `steganography.rs:645`, `steganography.rs:791`)

- JPEG stego uses `STEGO_JPEG_AMPLITUDE` (40), `STEGO_JPEG_SPREAD` (5), `STEGO_JPEG_BLOCK_STRIDE` (15) — **Confirmed** (`steganography.rs:784-786`)

- DCT stego for baseline JPEG (F5 + seed in Q-tables) — **Confirmed** (`steganography.rs:92-127`)

- Progressive JPEG: seed-in-Q-tables only — **Confirmed** (`steganography.rs:129-142`)

- HMAC uses `subtle::ConstantTimeEq::ct_eq()` — **Confirmed** (`steganography.rs:15` import, `steganography.rs:512` usage)

- `verify_payload_with_key()` returns `Option<bool>` — **Confirmed** (`steganography.rs:245`)

- Extraction always runs 5 passes — **Confirmed** (`steganography.rs:222`, `steganography.rs:846` EXTRACT_REDUNDANCY = 5)

- `FALLBACK_SEEDS` constant — **Confirmed** (`steganography.rs:25`): `[42, 0, 1, 12345, 99999, 123456789]`

- `protection_level()` returns `ProtectionLevel::Standard` — **Confirmed** (`steganography.rs:1016`)

### Discrepancies

1. **`MIN_PAYLOAD_SIZE` is 26, not 32.** Document says "Always padded to 32 bytes (`MIN_PAYLOAD_SIZE`). `MIN_PAYLOAD_BITS = 256`." Code says `MIN_PAYLOAD_SIZE = 26` and `MIN_PAYLOAD_BITS = 208`. The 32-byte padded output from `generate_payload` is the *output* size, not the minimum accepted size. This is a factual error in the document.

2. **Payload format "Reserved (zeroed)" field.** The document shows bytes 20-23 as a "Reserved (zeroed)" field. In reality, bytes 12-19 are the timestamp (u64 LE), and bytes 20-23 are zero because `generate_payload` pads to 24 bytes with zeros. There is no explicit "Reserved" semantic — it's just zero-padding to reach the 24-byte header boundary. The `parse_stego_payload` function (`steganography.rs:395-421`) does not read bytes 20-23 at all.

3. **Payload size: 24 header + 8 HMAC vs 24 header + 2 checksum.** Document says "HMAC-SHA256 (with key) or additive checksum (without key)" is 8 bytes at offset 24. The HMAC is indeed 8 bytes (first 8 bytes of HMAC-SHA256), but the checksum is only 2 bytes (`steganography.rs:522-526`). The document's "32 bytes total" and "24 header + 8 HMAC" is only accurate for the HMAC mode. In checksum mode, it's 24 header + 2 checksum + 6 padding = 32 bytes.

4. **`estimated_latency_ms()` return type.** Document shows `f64`, actual is `u32` returning `2`.

5. **`Payload format` description says `Intensity (u16, scaled f32)`.** The actual encoding is `(intensity * 100.0) as u16` (`steganography.rs:580`), and decoding is `intensity_raw as f32 / 100.0` (`steganography.rs:413`). This is confirmed but the document doesn't mention the scale factor.

### Improvement Opportunities

- None.

### Potential Bugs/Edge Cases

- **Payload size confusion.** `MIN_PAYLOAD_SIZE = 26` but `generate_payload` produces 32 bytes. The `verify_checksum` function checks `payload.len() < MIN_PAYLOAD_SIZE` (26), but `verify_payload_integrity` for HMAC mode checks `payload.len() >= 32`. This means a 26-byte payload (with 2-byte checksum) would pass checksum verification but fail HMAC verification even with a valid key. The asymmetry is by design but could cause confusion.

- **JPEG stego extraction reads all 5 passes regardless.** `extract_jpeg_stego` always runs `EXTRACT_REDUNDANCY` (5) passes even though embedding may have used fewer passes. This is intentional for robustness (majority voting), but means extraction is always 5x the work of a single-pass extraction.

---

## Cross-Cutting Findings

### `is_enabled()` is Dead Code

The `is_enabled()` method is defined in the `Protector` trait (`traits.rs:91`) with a default of `true`. `PassthroughProtector` is the only implementation that overrides it (returns `true`, not `false` as the document claims). No code in the pipeline calls `is_enabled()`. The method appears to be dead code or intended for future use. The `constants.md` claim that "the pipeline skips this protector entirely" via `is_enabled()` is incorrect — the pipeline uses direct `match level` dispatch.

### `estimated_latency_ms()` Type Mismatch in All Documents

Every architecture document shows the return type as `f64`, but the actual trait signature is `fn estimated_latency_ms(&self) -> u32`. The latency values in the documents are also inaccurate:
- Noise: doc says ~5.0, code returns 3
- Enhanced: doc says ~7.0, code returns 5
- MetadataTrap: doc says 2, code returns 2 (correct)
- Precomputed: doc says 0.0, code returns 2

### PrecomputedProtector Unbounded Cache

The `PrecomputedProtector` uses a `RwLock<HashMap>` with no size limit, eviction policy, or TTL. Under sustained load with diverse images, the cache will grow without bound. This is not documented anywhere. A production deployment would need an external eviction strategy (e.g., LRU via the `VariantLoader` trait, or memory limits).

### PrecomputedProtector Zero-Intensity Missing

The document claims `apply()` returns `Cow::Borrowed` when intensity is 0.0. The actual code does not have this optimization. The `NoiseProtector` does have this check, but `PrecomputedProtector` does not.

### Metadata Injection Survives Only in Byte Paths

Metadata injection (`MetadataTrapProtector`) operates on raw bytes and does not survive `DynamicImage` encode/decode cycles. The pipeline correctly routes `Light` level through `apply_light_bytes()` which encodes → injects → decodes. The metadata is preserved because injection happens AFTER encoding to bytes. However, if a consumer decodes the output `DynamicImage` back to bytes, the metadata should survive because `image::load_from_memory` preserves unknown chunks/markers.

### Stego Payload Size Constants Discrepancy

`MIN_PAYLOAD_SIZE = 26` (code) vs "32 bytes total" (document). The `generate_payload` function produces 32 bytes, but parsing/verification uses `MIN_PAYLOAD_SIZE = 26` as the lower bound. This means:
- A 26-byte payload (24 header + 2 checksum) is valid for checksum verification
- A 32-byte payload (24 header + 8 HMAC + 6 padding) is valid for HMAC verification
- Both are padded to 32 bytes by `generate_payload`
- The "32 bytes total" in the document describes the output format, not the minimum accepted size

### Thread-Safety: PrecomputedProtector Read Lock Contention

On cache hit, `get_cached_variant` holds a read lock while cloning the `ProtectedVariant` (which contains a `Vec<u8>` for perturbation data). Under high concurrency, this clone-under-lock could become a bottleneck. The two-phase registration design correctly avoids holding locks during I/O, but the read path still holds the lock during clone.

### DMI Value for Strong Level

Both `Light` and `Strong` protection levels auto-map to `DmiValue::Prohibited`. This means the IPTC metadata does not distinguish between the lightest and strongest protection levels. A consumer reading the DMI metadata cannot determine the actual protection strength applied.
