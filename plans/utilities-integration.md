# Utilities & Integration Review Findings

## Document: util-image.md

### Verified Claims

- **XorShiftRng XOR offset**: `XORSHIFT_SEED_OFFSET` constant is `0x123456789ABCDEF0` (`protected/constants.rs:21`), applied via `wrapping_add` in `XorShiftRng::new()` (`image.rs:47`). Doc claim confirmed.
- **parallel_threshold() formula**: Returns `cores * 64 * 64` where `cores = rayon::current_num_threads().max(1)` (`image.rs:577-579`). Doc claim confirmed.
- **SIN_TABLE**: Exists but has **1024 entries**, not 256 as the doc states. `SIN_TABLE_SIZE = 1024` at `image.rs:15`. The `fast_sin` function normalizes angles into `[0, TAU)` and indexes into this table (`image.rs:26-31`).
- **F5XorShiftRng warning**: Confirmed — doc correctly warns about non-interchangeability with the F5-specific PRNG.
- **PerturbationRuntime**: Doc correctly describes it as a shared setup struct that eliminates duplicated code between serial and parallel paths. Confirmed struct exists at `image.rs:219`.
- **compute_image_hash**: SHA-256 hex hash of RGBA pixel data. Confirmed at `image.rs:410-419`.
- **detect_image_format**: PNG/JPEG/WebP detection via magic bytes. Confirmed at `image.rs:424-431`.
- **encode_image**: Default JPEG quality is 90. Confirmed at `image.rs:438`.
- **Module interactions**: Claims about `protected/noise.rs`, `protected/precomputed.rs`, `protected/steganography.rs`, `util/seed.rs`, and `lib.rs` interactions are consistent with code structure.

### Discrepancies

- **NoiseGenerator struct fields**: Doc says `mac_key: Vec<u8>`. Actual code has `seed: u64, mac_key: Option<Arc<[u8]>>` (`image.rs:89-92`). The struct also has a `seed` field not mentioned in the doc.
- **NoiseGenerator API**: Doc says `new(key: &[u8])` and `derive_seed(&self, component: u8, extra: u64) -> u64`. Actual code has `new(seed: u64)`, `with_mac_key(seed: u64, mac_key: impl Into<Arc<[u8]>>)`, and `derive_keyed_seed(&self, pixel_pos: u64) -> u64`. Names, parameters, and types all differ.
- **PerturbationParams visibility**: Doc describes it as a public struct with public fields. Actual code has it as `struct PerturbationParams` (private, `image.rs:132`). Fields differ: doc lists `freq_h`, `freq_v`, `freq_d`, `amplitude`; code has `inv_pattern_scale`, `intensity_factor`, `phase_offset`, `blocks_x`, `keyed_seed_base`, `noise_gen`. Doc omits `blocks_x`, `keyed_seed_base`, `noise_gen`.
- **PerturbationParams fields**: Doc claims `intensity`, `block_width`, `block_height`, `keyed_seed_base`. Actual struct has `intensity`, `blocks_x` (not `block_width`/`block_height`), `keyed_seed_base`. No `block_width` or `block_height` fields exist.
- **XorShiftRng methods**: Doc says `next_u32()` and `next_u32_range(max: u32)`. Actual code has `next_u64()`, `gen_f32()`, `gen_range()`, `gen_range_usize()`. No `next_u32` or `next_u32_range` methods exist.
- **Function signatures — single-pass perturbation**: Doc says `apply_perturbation_single_pass(img: &mut RgbaImage, params: &mut PerturbationParams, ctx: &ProtectionContext) -> Vec<u8>`. Actual: `apply_perturbation_single_pass(img: &RgbaImage, seed: u64, intensity: f32, intensity_multiplier: f32) -> DynamicImage`. Takes `&RgbaImage` (immutable), different parameters, returns `DynamicImage`.
- **Function signatures — keyed single-pass**: Doc says `apply_perturbation_single_pass_keyed(img: &mut RgbaImage, params: &mut PerturbationParams, ctx: &ProtectionContext) -> Vec<u8>`. Actual: `apply_perturbation_single_pass_keyed(img: &RgbaImage, seed: u64, intensity: f32, intensity_multiplier: f32, mac_key: &[u8]) -> DynamicImage`. Same issues as above.
- **Function signatures — parallel perturbation**: Doc says `apply_perturbation_single_pass_keyed_par(img: &mut RgbaImage, params: &mut PerturbationParams, ctx: &ProtectionContext) -> Vec<u8>`. Actual: `apply_perturbation_single_pass_keyed_par(img: &RgbaImage, seed: u64, intensity: f32, intensity_multiplier: f32, mac_key: &[u8]) -> DynamicImage`. Same issues.
- **Function signatures — precomputed application**: Doc says `apply_perturbation(img: &mut RgbaImage, perturbation: &[u8], divisor: f32)` and `apply_perturbation_par(img: &mut RgbaImage, perturbation: &[u8], divisor: f32)`. Actual: both take `&RgbaImage` (immutable), `divisor` is `i16` (not `f32`), and both return `Result<RgbaImage>`. Doc omits return type.
- **PerturbationParams `derive_spatial_seed()`**: Doc says `PerturbationParams` exposes `derive_spatial_seed()`. Confirmed at `image.rs:172-174`, but `PerturbationParams` is private, so this is only accessible within the module.

### Improvement Opportunities

- Doc should be updated to reflect the actual function signatures, which are significantly different from what's documented.
- The `NoiseGenerator` documentation should describe the two constructors (`new` and `with_mac_key`) and the actual `derive_keyed_seed` method.
- The `XorShiftRng` method list should match the actual API (`next_u64`, `gen_f32`, `gen_range`, `gen_range_usize`).

### Potential Bugs/Edge Cases

- **SIN_TABLE size discrepancy**: If any code assumes 256 entries based on the doc, it would break. The actual table is 1024 entries. Not currently a bug since the doc is just informational, but misleading.
- **`apply_perturbation` divisor type**: The `divisor` parameter is `i16`, not `f32`. Integer division by `divisor` means division truncates toward zero. If `divisor` is 0, this would cause a division-by-zero panic. The code does not validate this.
- **`apply_perturbation` size mismatch**: Returns `Err` if perturbation buffer length doesn't match `width * height * 4`. The caller must ensure exact sizing.

---

## Document: util-iscc.md

### Verified Claims

- **Normalize to 32×32 grayscale**: Confirmed — `normalize_image()` converts to luma8 then resizes to 32×32 with Lanczos3 (`iscc.rs:48-53`).
- **2D DCT**: Confirmed — `compute_dct_2d()` performs a standard 2D DCT (`iscc.rs:69-113`).
- **Median-based bit pattern**: Confirmed — `dct_to_hash()` computes median of DCT coefficients and sets bits based on whether values exceed the median (`iscc.rs:116-154`).
- **SHA-256 data hash**: Confirmed — `compute_data_code()` hashes raw RGBA bytes with SHA-256 (`iscc.rs:157-165`).
- **Base58 encoding**: Confirmed — `encode_iscc_component()` uses `base58::ToBase58` (`iscc.rs:168-174`).
- **compute_iscc and compute_iscc_from_bytes**: Both exist and are public (`iscc.rs:178-188`).
- **Not in protection hot path**: Confirmed — ISCC is only used for content identification, not in the protection pipeline.

### Discrepancies

- **Iscc struct fields**: Doc says `pub meta: String`. Actual code has `pub meta: Option<String>` (`iscc.rs:12`). The field is optional.
- **compute_iscc_from_bytes return type**: Doc says `Result<Iscc>`. Actual code returns `Option<Iscc>` (`iscc.rs:185`).
- **64-bit perceptual hash claim**: The doc says the algorithm produces a "64-bit perceptual hash." Actual code produces a 256-bit (32-byte) hash via `dct_to_hash()` (`iscc.rs:116`). The function iterates over 4 quadrants of 8×8 blocks (4 × 63 = 252 coefficients after excluding DC), producing up to 252 bits, stored in a 32-byte array. The hash is then truncated to 8 bytes via `content_bytes()` for ISCC component encoding, but the full hash is 256 bits.
- **Instance hash description**: Doc says instance hash is "SHA-256 of the normalized grayscale pixels." Actual code computes `instance` as the same `data_code` (SHA-256 of raw RGBA bytes, not normalized grayscale) (`iscc.rs:36-38`). The `instance` field is simply a clone of `data`.
- **ISCC component codes**: Doc doesn't mention the header bytes. Actual code uses `0x12` for image content code and `0x33` for data code (`iscc.rs:66`, `iscc.rs:165`). These differ from the ISCC standard component type codes.

### Improvement Opportunities

- Doc should clarify that the full perceptual hash is 256 bits, truncated to 64 bits (8 bytes) for the ISCC content component.
- The instance hash description should note it is identical to the data hash (both are SHA-256 of raw RGBA bytes).
- Document the ISCC component header bytes (`0x12`, `0x33`) and how they map to the ISCC specification.

### Potential Bugs/Edge Cases

- **ISCC standard compatibility**: This implementation does NOT conform to the ISCC standard. The ISCC specification defines specific component type codes (e.g., `0x10` for Meta, `0x20` for Semi-isolated content, `0x30` for Data, `0x40` for Instance). This code uses `0x12` and `0x33` which don't match. The DCT hash uses 4 quadrants of 8×8 blocks from a 32×32 image, whereas the ISCC standard specifies a different coefficient selection. The result is a custom content identifier, not a standard-compliant ISCC code.
- **Collision rate**: No collision rate testing is present. The `test_iscc_deterministic` test only verifies that the same input produces the same output. There are no tests for hash uniqueness across different images or similarity of hashes for similar images.
- **DCT quadrant overlap**: The quadrants `[(0,0), (8,0), (0,8), (8,8)]` divide the 32×32 DCT into 4 non-overlapping 8×8 blocks. The first quadrant (0,0) starts at `x+y > 0`, excluding the DC coefficient. The other quadrants don't have this exclusion — they include the DC sub-band of their sub-block. This is inconsistent with typical perceptual hash implementations that exclude all DC coefficients.

---

## Document: util-seed.md

### Verified Claims

- **SystemTime as entropy source**: Confirmed — `std::time::SystemTime::now()` is used (`seed.rs:17`).
- **splitmix64 algorithm**: Partially confirmed. The actual code uses a splitmix64-like mixing function, but it's **not the standard splitmix64** from the doc's pseudocode. The doc shows a single `splitmix64(time)` call. The actual code (`seed.rs:20-27`) performs a custom mixing function: XOR with golden ratio constant, then three rounds of bit shifts and multiplications. This matches the standard splitmix64 algorithm's mixing constants (`0x9E3779B97F4A7C15`, `0xBF58476D1CE4E5B9`, `0x94D049BB133111EB`), but the structure is a direct implementation rather than a function call.
- **Not cryptographically secure**: Confirmed — doc correctly warns about predictability.
- **ProtectionContext::default() usage**: Confirmed — `ProtectionContext::default()` calls `generate_random_seed()` (`types.rs:322`).
- **Re-exported as public API**: Confirmed — `generate_random_seed` is imported and used in `lib.rs`.

### Discrepancies

- **unwrap() behavior**: Doc says `generate_random_seed().unwrap()` could panic if clock before UNIX epoch. Actual code uses `.unwrap_or_default()` (`seed.rs:18-19`), which returns a zero `Duration` instead of panicking. The clock-before-epoch concern is handled gracefully — it doesn't panic. The doc's panic claim is incorrect.
- **Non-zero guarantee**: The doc doesn't mention the non-zero guarantee. Actual code explicitly checks `if x == 0 { 42 }` (`seed.rs:28-30`), ensuring the seed is never zero.
- **Mixing details**: The doc's pseudocode shows `SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64` then `splitmix64(time)`. The actual code uses `now.as_secs()` (seconds, not nanoseconds) XOR'd with `now.subsec_nanos().wrapping_mul(0x9E3779B97F4A7C15)`, then applies the splitmix64 mixing. The entropy source and mixing are different from what the doc describes.

### Improvement Opportunities

- Doc should describe the actual mixing function rather than the simplified pseudocode.
- Doc should mention the non-zero guarantee.
- Doc should correct the `unwrap()` claim — the code uses `unwrap_or_default()` and does not panic.

### Potential Bugs/Edge Cases

- **Low entropy**: If two calls happen within the same second, `as_secs()` returns the same value. The nanosecond component is mixed via golden ratio multiplication, which provides some differentiation, but two calls at the exact same nanosecond would produce the same seed. The test only asserts non-zero, not uniqueness.
- **predictability**: The doc correctly warns about predictability, but the threat model is underspecified. An attacker who can observe the output image can potentially reconstruct the seed via brute-force given the small search space of plausible timestamps.

---

## Document: async-api.md

### Verified Claims

- **Behind async feature flag**: Confirmed — the module uses `tokio::task::spawn_blocking`.
- **Batch functions use single spawn_blocking**: Confirmed — `process_images_parallel_async` and `process_images_bytes_parallel_async` each call `spawn_blocking` once, delegating to `process_images_parallel` / `process_images_bytes_parallel` which use rayon internally (`async_api.rs:108-116`, `async_api.rs:125-133`).
- **Single-image functions use one spawn_blocking per image**: Confirmed — `process_image_async` and `process_image_bytes_async` each use one `spawn_blocking` call (`async_api.rs:76-84`, `async_api.rs:91-99`).
- **Ownership semantics**: Confirmed — all async functions take owned types (`Vec<u8>`, `DynamicImage`, `ProtectionContext`).
- **Error mapping**: Confirmed — `join_err` function maps `JoinError` to `Error::Task`, handling cancelled and panicked cases (`async_api.rs:62-70`).
- **Delegation to sync functions**: Confirmed — all async functions delegate to corresponding sync functions in `lib.rs`.

### Discrepancies

- **Return types**: Doc says `process_image_async` returns `Cow<'static, DynamicImage>`. Actual code returns `Result<DynamicImage>` (`async_api.rs:80`). The doc omits the `Result` wrapper and uses `Cow` instead of owned `DynamicImage`.
- **Return types — batch functions**: Doc says `process_images_parallel_async` returns `Vec<Cow<'static, DynamicImage>>`. Actual returns `Result<Vec<DynamicImage>>` (`async_api.rs:112`).
- **Return types — bytes functions**: Doc says `process_image_bytes_async` returns `Vec<u8>`. Actual returns `Result<Vec<u8>>` (`async_api.rs:95`).
- **Return types — verify**: Doc says `verify_image_bytes_async` returns `Option<bool>`. Actual returns `Result<Option<bool>>` (`async_api.rs:144`).

### Improvement Opportunities

- Doc should list the actual return types including `Result` wrappers.
- Doc should mention the `#[must_use]` attribute on all async functions.

### Potential Bugs/Edge Cases

- **Cancellation behavior**: The doc does not describe what happens when an async task is cancelled mid-execution. Since `spawn_blocking` runs synchronous code, cancellation only takes effect at the `.await` point — the blocking task will complete even if the future is dropped. The `join_err` function handles the cancelled case by returning `Error::Task`, but this only triggers if the task is cancelled after it has already started running.
- **Thread pool starvation**: If too many single-image async calls are made concurrently, they could exhaust the tokio blocking thread pool. The doc does not discuss backpressure or thread pool sizing recommendations.

---

## Document: cli.md

### Verified Claims

- **Built with clap 4 derive**: Confirmed — uses `#[derive(Parser)]` and `#[derive(ValueEnum)]` (`main.rs:10`, `main.rs:105`).
- **Options table**: The flags, long names, and default values match the code for most options. Specific checks:
  - `-o`/`--output`: Correct.
  - `-V`/`--verify`: Correct (default false).
  - `-l`/`--level`: Correct (default "standard").
  - `-i`/`--intensity`: Correct (default "0.5").
  - `-s`/`--seed`: Correct (Option<u64>).
  - `-f`/`--format`: Correct (Option<OutputFormatArg>).
  - `--stego-redundancy`: Correct (default "2").
  - `--jpeg-quality`: Correct (default "90").
  - `--progressive`: Correct (default false).
  - `-v`/`--verbose`: Correct (default false).
  - `-d`/`--dmi`: Correct (Option<DmiArg>).
  - `-k`/`--key`: Correct (Option<String>).
  - `-j`/`--jobs`: Correct (default "1").
- **Batch processing with rayon**: Confirmed — uses `rayon::par_iter()` when `jobs > 1` (`main.rs:451-467`).
- **Dependencies**: clap, cloakrs, image, rayon, hex — all confirmed in the code imports.
- **Module interactions**: Calls `process_image_bytes`, `process_images_bytes_parallel` (via `process_single_file`), and uses `verify_image_bytes` indirectly (though the CLI implements its own verification logic). Uses `ProtectionLevel`, `ProtectionContext`, `ImageOutputFormat`, `DmiValue`.

### Discrepancies

- **`--metadata` default**: Doc table says default is `false`. Actual code has `metadata: Option<bool>` with no default value (`main.rs:82`). When `None`, the library defaults to injecting metadata for non-Disabled levels (i.e., `true` for Standard+). The help text says "Default: true for Standard+, false for Light" which is more accurate than the doc's `false`. The doc's claim of `false` as the default is incorrect.
- **`--legal-claims` default**: Doc table says default is `false`. Actual code has `legal_claims: bool` with no `default_value` attribute, so clap defaults to `false`. Confirmed correct.
- **Verification mode extraction order**: Doc says "1. Extract seed from metadata (PNG tEXt, JPEG COM, WebP META), 2. Check DCT stego for JPEG images." Actual CLI code (`main.rs:305-330`) first tries metadata extraction, then falls back to `stego.extract_payload()` (LSB stego), **not** DCT stego. The library's `verify_payload_from_bytes_with_key` does DCT stego first for JPEG, then metadata. The CLI does NOT use this library function — it has its own verification logic. The doc's order doesn't match either the CLI code or the library code.
- **HMAC verification**: Doc says "5. Verify HMAC signature if key provided." Actual CLI code does NOT perform HMAC verification in the `-V` path. It only extracts metadata seed or falls back to LSB payload extraction. No HMAC key handling exists in the verification branch.
- **Format auto-detection priority**: Doc says "1. Check `--format` flag, 2. Check output file extension, 3. Detect from input magic bytes, 4. Default to input format." The actual code in `process_single_file` (`main.rs:213-224`) only checks `--format` flag, then falls back to detected format from magic bytes. There is **no output file extension check** — the output format is not derived from the output file extension. Step 2 from the doc is missing.
- **Output file extension**: Doc says `-o` with a file sets the output path directly. The actual code in `process_single_file` (`main.rs:236`) always generates the output filename as `{stem}_protected.{ext}` regardless of the `-o` argument. The `-o` argument for single files sets the output directory, not the output file path directly. This contradicts the doc's claim.
- **Stdin/stdout**: Doc says "Reads from stdin when no input files specified." The actual code (`main.rs:257-259`) exits with an error if no input files are found. There is no stdin reading implementation.

### Improvement Opportunities

- Doc should describe the actual verification logic flow: metadata seed extraction → LSB stego fallback.
- Doc should remove the claim about output file extension detection in format auto-detection.
- Doc should correct the stdin/stdout claim — the CLI requires input files and errors on empty input.
- Doc should document the actual behavior of `-o` for single files (generates `{stem}_protected.{ext}` in the specified directory).
- Doc should note that the CLI's verification mode does not support HMAC key verification, unlike the library's `verify_image_bytes` function.

### Potential Bugs/Edge Cases

- **Exit codes**: The CLI uses `std::process::exit(1)` for no input files (`main.rs:259`) and for batch verify (`main.rs:279`). For batch failures, it returns an `Err` which becomes exit code 1 via the `Box<dyn Error>` return. Exit code documentation is missing from the doc.
- **Rayon thread pool initialization**: The CLI calls `rayon::ThreadPoolBuilder::new().num_threads(jobs).build_global()` (`main.rs:426-429`) only when `jobs > 1`. If called after the global pool is already initialized (e.g., by a library dependency), this silently fails with a warning in verbose mode. The error is swallowed in non-verbose mode.
- **Output filename collision**: In batch mode, if two input files have the same stem, the `_protected.{ext}` filenames will collide, overwriting earlier outputs. No collision detection exists.

---

## Cross-Cutting Findings

1. **Structural documentation drift**: The `util-image.md` document is significantly out of date with the actual code. Nearly every public function signature, struct definition, and method list is incorrect. The document appears to have been written for an earlier version of the API and never updated.

2. **Return type consistency**: Multiple documents omit `Result` wrappers from return types (async-api.md, util-image.md). This could mislead users into thinking operations are infallible.

3. **ISCC is not ISCC-standard compliant**: The ISCC implementation uses custom component codes and a non-standard hash algorithm. It should not be described as "ISCC" without qualification — it produces ISCC-like identifiers that are not interoperable with other ISCC implementations.

4. **CLI vs library verification divergence**: The CLI implements its own verification logic that differs from the library's `verify_image_bytes` / `verify_payload_from_bytes_with_key`. The library does DCT-first for JPEG; the CLI does metadata-first for all formats. The CLI also lacks HMAC verification that the library supports. These divergences are not documented.

5. **Missing documentation topics**: Neither the async-api.md nor cli.md documents cancellation behavior, exit codes, or thread pool sizing recommendations. These are important for production WAF/CDN deployment.

6. **SIN_TABLE size mismatch**: The doc claims 256 entries; the code has 1024. While this doesn't cause bugs (the table is internal), it indicates the documentation was not updated when the table was resized.

7. **`unwrap_or_default()` vs `unwrap()`**: The seed generation doc claims `unwrap()` could panic on pre-UNIX-epoch clocks, but the code uses `unwrap_or_default()`. This is a factual error in the documentation.

8. **NoiseGenerator constructor mismatch**: The doc describes a constructor `new(key: &[u8])` that doesn't exist. The actual API has `new(seed: u64)` and `with_mac_key(seed: u64, mac_key: ...)`. This is the most severe documentation discrepancy, as it describes a completely different API surface.
