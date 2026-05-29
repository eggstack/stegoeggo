# Consolidated Architecture Fix Plan

## Overview

This plan consolidates findings from 6 architecture review files into a single actionable document. The codebase has **50+ documentation discrepancies**, **15+ potential bugs/edge cases**, and **50 stale documentation items**. The fixes are organized into 5 waves to maximize parallel execution via sub-agents.

**Key principle**: Documentation must match code exactly. When docs say a type is `Vec<u8>` but code says `Option<Vec<u8>>`, users will fail to compile.

---

## Wave 1: Critical Type & Signature Fixes (Parallel)

**Goal**: Fix all discrepancies that would cause compilation failures for users following the docs. These are independent files that can be fixed simultaneously.

### Task 1A: Fix `types.md` discrepancies
**File**: `architecture/types.md`
**Changes**:
1. Fix `DmiValue` variant names: `ProhibitedScraping` → `ProhibitedExceptSearchEngineIndexing`, `ProhibitedAnyProcessing` → `ProhibitedSeeConstraints` (code: `types.rs:8-17`)
2. Fix `ProtectionConfig.mac_key` type: `Vec<u8>` → `Option<Vec<u8>>` (code: `types.rs:269`)
3. Fix `ProtectionContext.inject_metadata` type: `bool` → `Option<bool>`, default `true` → `None` (code: `types.rs:304`)
4. Fix `ProtectionContext.inject_legal_claims` type: `bool` → `Option<bool>`, default `false` → `None` (code: `types.rs:305`)
5. Fix `ProtectionContext.stego_redundancy` type: `u8` → `usize` (code: `types.rs:306`)
6. Fix `ProtectedVariant` field: `uuid: Uuid` → `variant_id: uuid::Uuid` (code: `types.rs:562`)
7. Fix `ProtectedVariant::cache_key()` format: `{uuid}_{hash}_{intensity}` → `{hash}_{level}_{intensity}` (code: `types.rs:600-606`)
8. Fix `StegoPayload::protection_level()` return type: `ProtectionLevel` → `u8` (code: `steganography.rs:1038`)
9. Add note: `None` for `inject_metadata`/`inject_legal_claims` means "use level default"; explicit `false` disables injection

### Task 1B: Fix `overview.md` discrepancies
**File**: `architecture/overview.md`
**Changes**:
1. Fix `DmiValue` variant names: same as Task 1A item 1
2. Note that Light level encodes and re-decodes image via `apply_light_bytes()` (code: `lib.rs:290-303`)

### Task 1C: Fix `protected-steganography.md` discrepancies
**File**: `architecture/protected-steganography.md`
**Changes**:
1. Fix all 5 public method signatures from free functions to `&self` methods:
   - `extract_payload(img, ctx)` → `extract_payload(&self, img)` (code: `steganography.rs:360`)
   - `verify_payload(img, ctx)` → `verify_payload(&self, img)` (code: `steganography.rs:68`)
   - `verify_payload_with_key(img, ctx, key)` → `verify_payload_with_key(&self, img, mac_key)` (code: `steganography.rs:245`)
   - `verify_payload_from_bytes(bytes, ctx)` → `verify_payload_from_bytes(&self, img_bytes, seed)` (code: `steganography.rs:294`)
   - `verify_payload_from_bytes_with_key(bytes, ctx, key)` → `verify_payload_from_bytes_with_key(&self, img_bytes, mac_key)` (code: `steganography.rs:257`)
2. Fix `MIN_PAYLOAD_SIZE`: 32 → 26 (code: `steganography.rs:20`)
3. Fix `MIN_PAYLOAD_BITS`: 256 → 208 (code: `steganography.rs:22`)
4. Fix payload format "Reserved (zeroed)" field — it's just zero-padding, not a named field
5. Note checksum is 2 bytes (not 8), HMAC is 8 bytes

### Task 1D: Fix `jpeg-transcoder.md` discrepancies
**File**: `architecture/jpeg-transcoder.md`
**Changes**:
1. Fix `Coefficients` inner type: `[i64; 64]` → `[i16; 64]` (code: `mod.rs:14`)
2. Fix `assemble_jpeg` visibility: `pub fn` → `fn` (private) (code: `mod.rs:93`)
3. Fix `assemble_jpeg` return type: `Vec<u8>` → `Result<Vec<u8>>` (code: `mod.rs:93`)
4. Fix `get_scan_data_start` return type: `Result<usize>` → `Option<usize>` (code: `mod.rs:233`)
5. Fix `TranscoderError` variants — replace `InvalidData`, `UnsupportedFeature`, `EncodingError` with actual 6 variants: `InvalidFormat`, `Unsupported`, `HuffmanDecode`, `HuffmanEncode`, `Io`, `EmbeddingFailed` (code: `mod.rs:17-35`)
6. Fix `HuffmanEncoderTable` field name: `symbols` → `entries` (code: `entropy.rs:77`)

### Task 1E: Fix `util-image.md` discrepancies
**File**: `architecture/util-image.md`
**Changes**:
1. Fix `NoiseGenerator` struct: add `seed: u64` field, change `mac_key: Vec<u8>` → `mac_key: Option<Arc<[u8]>>` (code: `image.rs:89-92`)
2. Fix `NoiseGenerator::new`: `new(key: &[u8])` → `new(seed: u64)` (code: `image.rs:95`)
3. Add `NoiseGenerator::with_mac_key(seed: u64, mac_key: impl Into<Arc<[u8]>>)` constructor (code: `image.rs:102`)
4. Fix `NoiseGenerator::derive_seed` → `derive_keyed_seed(&self, pixel_pos: u64)` (code: `image.rs:109`)
5. Fix `PerturbationParams` — private struct, fields: `intensity`, `blocks_x`, `keyed_seed_base`, `inv_pattern_scale`, `intensity_factor`, `phase_offset`, `noise_gen` (code: `image.rs:132-140`)
6. Fix `XorShiftRng` methods: `next_u32()`/`next_u32_range(max)` → `next_u64()`, `gen_f32()`, `gen_range()`, `gen_range_usize()` (code: `image.rs:39-82`)
7. Fix `apply_perturbation_single_pass` signature: `(img: &mut RgbaImage, params: &mut PerturbationParams, ctx: &ProtectionContext)` → `(img: &RgbaImage, seed: u64, intensity: f32, intensity_multiplier: f32) -> DynamicImage` (code: `image.rs:313-320`)
8. Fix `apply_perturbation_single_pass_keyed` signature: same pattern, add `mac_key: &[u8]` param (code: `image.rs:323-350`)
9. Fix `SIN_TABLE_SIZE`: 256 → 1024 (code: `image.rs:15`)
10. Fix `apply_perturbation` and `apply_perturbation_par`: `&mut RgbaImage` → `&RgbaImage`, `divisor: f32` → `divisor: i16`, add `Result<RgbaImage>` return type

---

## Wave 2: Trait & Protection Strategy Fixes (Parallel)

**Goal**: Fix all protection strategy documentation and trait discrepancies. These are independent files.

### Task 2A: Fix `traits.md` discrepancies
**File**: `architecture/traits.md`
**Changes**:
1. Fix `estimated_latency_ms` return type: `f64` → `u32` (code: `traits.rs:88`)
2. Fix `VariantLoader::load_variant` return type: `Option<ProtectedVariant>` → `Result<Option<ProtectedVariant>>` (code: `traits.rs:108`)
3. Fix Protector implementations table latency values:
   - PassthroughProtector: 0 (code: `passthrough.rs:42`)
   - MetadataTrapProtector: 2 (code: `metadata_trap.rs:564`)
   - NoiseProtector: 3 (code: `noise.rs:83`)
   - EnhancedProtector: 5 (code: `enhanced.rs:49`)
   - SteganographyProtector: 2 (code: `steganography.rs:1019`)
   - PrecomputedProtector: 2 (code: `precomputed.rs`)
4. Fix `is_enabled()` description: return value is `true` (not `false` for Passthrough), and the method is dead code — pipeline never calls it (code: `passthrough.rs:46`, `traits.rs:91`)

### Task 2B: Fix `protected-passthrough.md` discrepancies
**File**: `architecture/protected-passthrough.md`
**Changes**:
1. Fix `is_enabled()` claim: returns `true`, not `false` (code: `passthrough.rs:46`)
2. Fix `estimated_latency_ms` return type: `f64` → `u32` (code: `passthrough.rs:42`)

### Task 2C: Fix `protected-noise.md` discrepancies
**File**: `architecture/protected-noise.md`
**Changes**:
1. Fix `estimated_latency_ms`: `~5.0` (f64) → `3` (u32) (code: `noise.rs:83-85`)

### Task 2D: Fix `protected-enhanced.md` discrepancies
**File**: `architecture/protected-enhanced.md`
**Changes**:
1. Fix `estimated_latency_ms`: `~7.0` (f64) → `5` (u32) (code: `enhanced.rs:49-51`)

### Task 2E: Fix `protected-precomputed.md` discrepancies
**File**: `architecture/protected-precomputed.md`
**Changes**:
1. Fix `estimated_latency_ms` return type: `f64` → `u32`, value: `0.0` → `2` (code: `precomputed.rs`)
2. Fix `cache_key` format: `{uuid}_{hash}_{intensity}` → `{hash}_{level}_{intensity}` (code: `precomputed.rs:114-121`)
3. Remove claim that `apply()` returns `Cow::Borrowed` at zero intensity — it doesn't have this check (code: `precomputed.rs:232-269`)
4. Fix `generate_perturbation_data` signature: add `&self` and `Result` return (code: `precomputed.rs:157-187`)
5. Fix `register_variant` — errors are propagated with `?`, not silently ignored (code: `precomputed.rs:65-78`)
6. Add warning about unbounded cache growth (no eviction policy)

### Task 2F: Fix `protected-metadata-trap.md` discrepancies
**File**: `architecture/protected-metadata-trap.md`
**Changes**:
1. Fix `estimated_latency_ms` return type: `f64` → `u32` (code: `metadata_trap.rs:564`)
2. Note that Strong DMI maps to `Prohibited` (same as Light), not a stronger variant

---

## Wave 3: Utility & JPEG Module Fixes (Parallel)

**Goal**: Fix utility module and JPEG transcoder sub-module documentation.

### Task 3A: Fix `util-iscc.md` discrepancies
**File**: `architecture/util-iscc.md`
**Changes**:
1. Fix `Iscc.meta` type: `String` → `Option<String>` (code: `iscc.rs:12`)
2. Fix `compute_iscc_from_bytes` return type: `Result<Iscc>` → `Option<Iscc>` (code: `iscc.rs:185`)
3. Clarify ISCC is NOT standard-compliant: uses custom component codes (`0x12`, `0x33`) and non-standard DCT hash (code: `iscc.rs:66`, `iscc.rs:165`)
4. Note perceptual hash is 256 bits, truncated to 64 bits for content component
5. Note instance hash is identical to data hash (both SHA-256 of raw RGBA bytes)

### Task 3B: Fix `util-seed.md` discrepancies
**File**: `architecture/util-seed.md`
**Changes**:
1. Fix `unwrap()` claim → `unwrap_or_default()` (code: `seed.rs:19`), does NOT panic on pre-UNIX-epoch clocks
2. Add non-zero guarantee: `if x == 0 { 42 }` (code: `seed.rs:28-30`)
3. Fix mixing description: uses `as_secs()` (seconds) XOR'd with `nanos * φ`, then splitmix64 mixing — not simple `splitmix64(as_nanos())`

### Task 3C: Fix `async-api.md` discrepancies
**File**: `architecture/async-api.md`
**Changes**:
1. Fix all return types to include `Result` wrapper:
   - `process_image_async`: `Cow<'static, DynamicImage>` → `Result<DynamicImage>` (code: `async_api.rs:80`)
   - `process_image_bytes_async`: `Vec<u8>` → `Result<Vec<u8>>` (code: `async_api.rs:95`)
   - `process_images_parallel_async`: `Vec<Cow<'static, DynamicImage>>` → `Result<Vec<DynamicImage>>` (code: `async_api.rs:112`)
   - `process_images_bytes_parallel_async`: `Result<Vec<Vec<u8>>>` (code: `async_api.rs:129`)
   - `verify_image_bytes_async`: `Option<bool>` → `Result<Option<bool>>` (code: `async_api.rs:144`)

### Task 3D: Fix `jpeg-header.md` discrepancies
**File**: `architecture/jpeg-header.md`
**Changes**:
1. Fix `quantization_tables` type: `Vec<QuantizationTable>` → `[Option<QuantizationTable>; 4]` (code: `header.rs:83`)
2. Fix `huffman_tables` organization: single `Vec` → two separate Vecs `huffman_tables_dc` and `huffman_tables_ac` (code: `header.rs:84-85`)
3. Fix `app_markers` type: `Vec<Vec<u8>>` → `app0_marker: Option<Vec<u8>>` and `app1_marker: Option<Vec<u8>>` (code: `header.rs:89-90`)
4. Fix `HuffmanTable.class` → `table_class: u8` (code: `header.rs:49`)
5. Fix `HuffmanTable.counts` type: `[u8; 16]` → `[u16; 16]` (code: `header.rs:51`)

### Task 3E: Fix `jpeg-stego-f5.md` discrepancies
**File**: `architecture/jpeg-stego-f5.md`
**Changes**:
1. Fix `embed_f5` return type: `Result<()>` → `Result<usize>` (code: `stego_f5.rs:178-183`)
2. Fix `extract_f5` return type: `Result<Vec<u8>>` → `Vec<u8>` (code: `stego_f5.rs:317-322`)
3. Fix `embed_seed_in_quantization_tables` — it's an `&self` method, not a free function (code: `stego_f5.rs:74-78`)
4. Fix `extract_seed_from_quantization_tables` — same, `&self` method (code: `stego_f5.rs:119`)
5. Correct 5-pass extraction claim — the 5-pass logic is in `steganography.rs:222` (`extract_with_redundancy`), not in F5 extraction. F5 extraction uses redundancy-based majority voting in a single pass (code: `stego_f5.rs:357-371`)

---

## Wave 4: Pipeline & CLI Fixes (Parallel)

**Goal**: Fix pipeline documentation and CLI documentation. These are independent.

### Task 4A: Fix `pipeline.md` discrepancies
**File**: `architecture/pipeline.md`
**Changes**:
1. Add note that `process_image_bytes` convenience function auto-detects format from magic bytes and sets `input_format` on context (code: `lib.rs:444-453`)
2. Document dimension validation asymmetry: `process()` validates dimensions (code: `lib.rs:217`) but `process_bytes()` does not (code: `lib.rs:318`)
3. Clarify JPEG fast path only triggers when BOTH input AND output are JPEG (code: `lib.rs:364-366`)
4. Fix `verify_image_bytes` — it's a free function, not a pipeline method (code: `lib.rs`)

### Task 4B: Fix `cli.md` discrepancies
**File**: `architecture/cli.md`
**Changes**:
1. Fix `-o` behavior: it sets output DIRECTORY, not output file path. Output filename is always `{stem}_protected.{ext}` (code: `main.rs:236-247`)
2. Remove stdin reading claim — CLI exits with error on empty input (code: `main.rs:255-259`)
3. Fix verification mode: does NOT do HMAC verification, only metadata seed extraction → LSB stego fallback (code: `main.rs:275-349`)
4. Fix format auto-detection: no output file extension check, only `--format` flag → input magic bytes → default PNG (code: `main.rs:210-224`)
5. Fix `--metadata` default: `None` (not `false`), library defaults to true for Standard+ (code: `main.rs:78-82`)
6. Note rayon thread pool init fails silently if already initialized (code: `main.rs:423-437`)

### Task 4C: Fix `pipeline.md` flow description
**File**: `architecture/pipeline.md`
**Changes**:
1. Correct Standard/Enhanced/Strong flow: perturbation → stego → encode → metadata is for the non-JPEG path. JPEG→JPEG path uses DCT stego directly without perturbation (code: `lib.rs:364-369`)
2. Document that `apply_light_bytes()` encodes → injects metadata → decodes, which can alter format/quality (code: `lib.rs:290-303`)

---

## Wave 5: Cleanup & Validation

**Goal**: Remove stale content, deduplicate, and verify all fixes compile.

### Task 5A: Remove stale items from architecture docs
**Files**: All docs
**Changes**:
1. Remove dead `is_enabled()` claims in passthrough docs
2. Remove duplicate XorShiftRng warnings (keep one in overview.md only)
3. Remove outdated cross-references in stale-items.md
4. Remove superseded DmiValue variant names
5. Remove incomplete module claims (CLI stdin, HMAC verification in CLI)

### Task 5B: Update `stale-items.md`
**File**: `plans/stale-items.md`
**Changes**:
1. Mark all items as addressed after their respective waves complete
2. Remove items that have been fixed

### Task 5C: Verify compilation
**Command**: `cargo check && cargo clippy --all-targets -- -D warnings && cargo test`
**Purpose**: Ensure no code changes were accidentally introduced (docs-only changes should not affect compilation)

### Task 5D: Verify documentation accuracy
**Method**: For each fixed item, grep the codebase to confirm the doc now matches code

---

## Known Bugs & Edge Cases (Not Documentation Fixes)

These are actual code issues identified during the review. They require code changes and should be tracked separately:

### Bug 1: `process_bytes` skips dimension validation
- **Impact**: Large images exceeding `max_dimension` bypass the check via byte path
- **Code**: `lib.rs:318-344` — no `validate_dimensions` call
- **Fix**: Add dimension validation to `process_bytes` or document the asymmetry

### Bug 2: PrecomputedProtector unbounded cache
- **Impact**: Memory exhaustion under sustained load with diverse images
- **Code**: `precomputed.rs:37` — `RwLock<HashMap>` with no eviction
- **Fix**: Add LRU eviction, max size, or TTL; document limitation

### Bug 3: Seed embedding silent failure
- **Impact**: Incorrect seed extraction when quantization table values are all 1
- **Code**: `stego_f5.rs:103-108` — `&= 0xFE` on value 1 produces 0, clamped to 1
- **Fix**: Return error or warning when embedding fails

### Bug 4: `inject_metadata`/`inject_legal_claims` `Option<bool>` ambiguity
- **Impact**: Callers cannot distinguish "not set" from "explicitly disabled"
- **Code**: `types.rs:304-305`
- **Fix**: Document semantics clearly or change API

### Bug 5: CLI output filename collision in batch mode
- **Impact**: Two files with same stem overwrite each other
- **Code**: `main.rs:236-247` — no collision detection
- **Fix**: Add collision detection or unique suffix

---

## Execution Strategy

### Parallelization by Wave

| Wave | Tasks | Dependencies | Estimated Parallelism |
|------|-------|-------------|----------------------|
| Wave 1 | 5 tasks (1A-1E) | None | All 5 in parallel |
| Wave 2 | 6 tasks (2A-2F) | None | All 6 in parallel |
| Wave 3 | 5 tasks (3A-3E) | None | All 5 in parallel |
| Wave 4 | 3 tasks (4A-4C) | None | All 3 in parallel |
| Wave 5 | 4 tasks (5A-5D) | Waves 1-4 | Sequential (validation) |

### Sub-Agent Usage

Each task can be assigned to a sub-agent. Maximum 4-5 tasks per sub-agent to avoid context compaction. Recommended grouping:

**Sub-agent 1**: Tasks 1A + 1B (types.md + overview.md — both have DmiValue fixes)
**Sub-agent 2**: Tasks 1C + 1D (steganography + jpeg-transcoder — both have signature fixes)
**Sub-agent 3**: Task 1E (util-image.md — many changes, dedicated agent)
**Sub-agent 4**: Tasks 2A + 2B + 2C + 2D (traits + passthrough + noise + enhanced — all latency fixes)
**Sub-agent 5**: Tasks 2E + 2F (precomputed + metadata-trap — remaining protection strategies)
**Sub-agent 6**: Tasks 3A + 3B + 3C (util-iscc + seed + async-api — utility modules)
**Sub-agent 7**: Tasks 3D + 3E (jpeg-header + jpeg-stego-f5 — JPEG sub-modules)
**Sub-agent 8**: Tasks 4A + 4B + 4C (pipeline + cli — pipeline docs)

### Verification

After each wave, run:
```bash
cargo check && cargo clippy --all-targets -- -D warnings
```

After Wave 5, run full test suite:
```bash
cargo test --all-features
```

---

## File Reference

All line numbers reference the current codebase state as of the review. Key files:
- `src/types.rs` — Core types (DmiValue, ProtectionConfig, ProtectionContext, ProtectedVariant)
- `src/traits.rs` — Protector trait definition
- `src/lib.rs` — Pipeline implementation
- `src/steganography.rs` — Steganography protector and payload handling
- `src/jpeg_transcoder/mod.rs` — JPEG transcoder entry
- `src/jpeg_transcoder/header.rs` — JPEG header parser
- `src/jpeg_transcoder/entropy.rs` — Huffman entropy codec
- `src/jpeg_transcoder/stego_f5.rs` — F5 DCT steganography
- `src/util/image.rs` — Noise generation, perturbation, encoding
- `src/util/iscc.rs` — ISCC content identifiers
- `src/util/seed.rs` — Random seed generation
- `src/async_api.rs` — Async wrappers
- `src/bin/cloakrs/main.rs` — CLI implementation
- `src/protected/passthrough.rs` — Disabled level protector
- `src/protected/noise.rs` — Standard level protector
- `src/protected/enhanced.rs` — Enhanced level protector
- `src/protected/precomputed.rs` — Strong level protector
- `src/protected/metadata_trap.rs` — Light level protector
