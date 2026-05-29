# Stale Architecture Items

## Dead References

- **traits.md:23** — `apply` signature documented as `fn apply(&self, img: &DynamicImage, ctx: &ProtectionContext) -> Cow<DynamicImage>` but actual code is `fn apply<'a>(&self, img: &'a DynamicImage, ctx: &ProtectionContext) -> Result<Cow<'a, DynamicImage>>` — missing `Result` wrapper and lifetime parameter
- **traits.md:82** — `name` return type documented as `&str` but actual code returns `&'static str`
- **traits.md:33-39 (Protector table)** — `estimated_latency_ms` documented as returning `f64` but actual trait signature returns `u32`
- **traits.md:28** — `PassthroughProtector.is_enabled()` documented as returning `false` but actual code returns `true`
- **protected-passthrough.md:20** — `is_enabled()` documented as returning `false` but actual code returns `true`
- **protected-steganography.md:69-73** — `extract_payload`, `verify_payload`, `verify_payload_with_key`, `verify_payload_from_bytes`, `verify_payload_from_bytes_with_key` documented as free functions but all are methods on `SteganographyProtector` (`&self` methods)
- **protected-steganography.md:69** — `extract_payload` signature documented as `fn extract_payload(img: &DynamicImage, ctx: &ProtectionContext) -> Option<StegoPayload>` but actual signature is `fn extract_payload(&self, img: &DynamicImage) -> Option<StegoPayload>` — takes `&self`, no `ctx`
- **protected-steganography.md:70** — `verify_payload` signature documented as `fn verify_payload(img: &DynamicImage, ctx: &ProtectionContext) -> bool` but actual is `fn verify_payload(&self, img: &DynamicImage) -> bool` — takes `&self`, no `ctx`
- **protected-steganography.md:71** — `verify_payload_with_key` signature documented as `fn verify_payload_with_key(img: &DynamicImage, ctx: &ProtectionContext, key: &[u8]) -> Option<bool>` but actual is `fn verify_payload_with_key(&self, img: &DynamicImage, mac_key: &[u8]) -> Option<bool>` — takes `&self`, no `ctx`
- **protected-steganography.md:72** — `verify_payload_from_bytes` documented as `fn verify_payload_from_bytes(bytes: &[u8], ctx: &ProtectionContext) -> bool` but actual is `fn verify_payload_from_bytes(&self, img_bytes: &[u8], seed: u64) -> bool` — takes `&self`, uses `seed` not `ctx`
- **protected-steganography.md:73** — `verify_payload_from_bytes_with_key` documented as `fn verify_payload_from_bytes_with_key(bytes: &[u8], ctx: &ProtectionContext, key: &[u8]) -> Option<bool>` but actual is `fn verify_payload_from_bytes_with_key(&self, img_bytes: &[u8], mac_key: &[u8]) -> Option<bool>` — takes `&self`, no `ctx`
- **protected-steganography.md:130** — `StegoPayload::protection_level()` documented as returning `ProtectionLevel` but actual code returns `u8`
- **jpeg-transcoder.md:17** — `assemble_jpeg` documented as `pub fn assemble_jpeg(header: &JpegHeader, scan_data: &[u8]) -> Vec<u8>` but actual signature is `fn assemble_jpeg(header: &JpegHeader, scan_data: &[u8]) -> Result<Vec<u8>>` — private method, returns `Result`
- **jpeg-transcoder.md:14** — `Coefficients` type documented as `HashMap<u8, Vec<[i64; 64]>>` but actual code is `HashMap<u8, Vec<[i16; 64]>>` — `i64` vs `i16`
- **jpeg-transcoder.md:40-43** — `HuffmanEncoderTable` field documented as `symbols: [(u16, u8); 256]` but actual code is `entries: [(u16, u8); 256]`
- **jpeg-transcoder.md:48** — `get_scan_data_start` documented as returning `Result<usize>` but actual code returns `Option<usize>`
- **jpeg-transcoder.md:64-68** — `TranscoderError` variants `InvalidData`, `UnsupportedFeature`, `EncodingError` do not exist; actual variants are `InvalidFormat`, `Unsupported`, `HuffmanDecode`, `HuffmanEncode`, `EmbeddingFailed`, `Io`
- **jpeg-header.md:83** — `JpegHeader.app_markers: Vec<Vec<u8>>` documented but actual fields are `app0_marker: Option<Vec<u8>>` and `app1_marker: Option<Vec<u8>>`
- **jpeg-header.md:61-62** — `HuffmanTable.class: u8` and `counts: [u8; 16]` documented but actual fields are `table_class: u8` and `counts: [u16; 16]`
- **util-image.md:55-56** — `apply_perturbation_single_pass` and `apply_perturbation_single_pass_keyed` documented as taking `(img: &mut RgbaImage, params: &mut PerturbationParams, ctx: &ProtectionContext)` but actual signatures are `(img: &RgbaImage, seed: u64, intensity: f32, intensity_multiplier: f32)` and `(img: &RgbaImage, seed: u64, intensity: f32, intensity_multiplier: f32, mac_key: &[u8])` — completely different parameter lists
- **util-image.md:65** — `apply_perturbation_single_pass_keyed_par` documented as taking `(img: &mut RgbaImage, params: &mut PerturbationParams, ctx: &ProtectionContext)` but actual signature is `(img: &RgbaImage, seed: u64, intensity: f32, intensity_multiplier: f32, mac_key: &[u8]) -> DynamicImage`
- **util-image.md:74-75** — `apply_perturbation` and `apply_perturbation_par` documented as taking `divisor: f32` but actual code uses `divisor: i16` and returns `Result<RgbaImage>` (not mutating in-place)
- **util-image.md:86** — `encode_image` documented as `encode_image(img, format) -> Vec<u8>` but actual signature is `fn encode_image(img: &DynamicImage, format: ImageFormat) -> Result<Vec<u8>>`
- **util-image.md:84** — `detect_image_format` documented as returning `Option<ImageOutputFormat>` but actual code returns `Option<ImageFormat>` (the `image` crate type)
- **util-image.md:26-27** — `NoiseGenerator` struct documented with `mac_key: Vec<u8>` but actual field is `mac_key: Option<Arc<[u8]>>`
- **util-image.md:29** — `NoiseGenerator::new` documented as `new(key: &[u8])` but actual code is `new(seed: u64)` — takes a seed, not a key
- **util-image.md:89** — `SIN_TABLE` documented as 256 entries but actual code defines `SIN_TABLE_SIZE = 1024`
- **util-image.md:36-37** — `PerturbationParams` documented fields `block_width`, `block_height`, `keyed_seed_base`, `freq_h`, `freq_v`, `freq_d`, `amplitude` do not match actual fields `blocks_x`, `inv_pattern_scale`, `intensity_factor`, `phase_offset`, `noise_gen`
- **util-iscc.md:10-16** — `Iscc.meta` documented as `String` but actual type is `Option<String>`
- **protected-noise.md:52** — Claims `NOISE_INTENSITY_MULTIPLIER` is referenced in `util/seed.rs` but it is only referenced in `protected/noise.rs` and `protected/constants.rs`
- **protected-noise.md:43** — `estimated_latency_ms` documented as `~5.0` (f64) but actual code returns `3` (u32)
- **protected-enhanced.md:40** — `estimated_latency_ms` documented as `~7.0` but actual code returns `5`
- **protected-metadata-trap.md:105** — Claims `STEGO_OFFSET_SEED_1` is used in `metadata_trap.rs` but it is only used in `steganography.rs` and `constants.rs`

## Superseded Information

- **types.md:42-53 (DmiValue variants)** — Documents seven variants (`Unspecified`, `Allowed`, `Prohibited`, `ProhibitedAiMlTraining`, `ProhibitedGenAiMlTraining`, `ProhibitedScraping`, `ProhibitedAnyProcessing`) but actual enum has seven variants: `Unspecified`, `Allowed`, `ProhibitedAiMlTraining`, `ProhibitedGenAiMlTraining`, `ProhibitedExceptSearchEngineIndexing`, `Prohibited`, `ProhibitedSeeConstraints` — `ProhibitedScraping` and `ProhibitedAnyProcessing` do not exist; `ProhibitedExceptSearchEngineIndexing` and `ProhibitedSeeConstraints` are undocumented
- **types.md:46-51 (DmiValue IPTC mapping)** — Documented IPTC property names use underscore (`DMI_Allowed`, `DMI_Prohibited`, `DMI_ProhibitedAiMlTraining`, etc.) but actual code uses dash (`DMI-Allowed`, `DMI-Prohibited`). Also, multiple DMI values map to the same IPTC property `Iptc4xmpExt:DMI-Prohibited` (not separate properties as implied by the table)
- **types.md:98-103 (ProtectionConfig)** — `mac_key` documented as `Vec<u8>` but actual type is `Option<Vec<u8>>`
- **types.md:78** — `stego_redundancy` documented as `u8` but actual type is `usize`
- **types.md:76-77** — `inject_metadata` and `inject_legal_claims` documented as `bool` but actual types are `Option<bool>`
- **protected-precomputed.md:50** — `generate_perturbation_data` signature documented as `fn generate_perturbation_data(width: u32, height: u32, ctx: &ProtectionContext) -> Vec<u8>` but actual code is `fn generate_perturbation_data(&self, width: u32, height: u32, ctx: &ProtectionContext) -> Result<Vec<u8>>` — takes `&self`, returns `Result`
- **protected-precomputed.md:33-42** — `register_variant` documented as silently ignoring loader errors (`let _ = loader.store_variant(&variant)`) but actual code propagates errors with `?`
- **protected-precomputed.md:77** — `cache_key` format documented as `{uuid}_{hash}_{intensity}` but actual code generates `{hash}_{level}_{intensity}` (no UUID, includes protection level)
- **types.md:126** — `ProtectedVariant.cache_key()` documented as returning `{uuid}_{hash}_{intensity}` but actual code returns `{hash}_{level}_{intensity}` — includes protection level, omits UUID
- **types.md:116-123** — `ProtectedVariant` struct fields listed as `uuid`, `original_hash`, `perturbation_data`, `intensity`, `width`, `height` but actual fields are `variant_id`, `original_hash`, `protection_level`, `perturbation_data`, `intensity`, `width`, `height` — `uuid` renamed to `variant_id`, `protection_level` field is undocumented
- **util-seed.md:13-18** — `generate_random_seed` documented as using `splitmix64(time)` but actual code uses a different algorithm: `s ^ (ns * 0x9E3779B97F4A7C15)` followed by three rounds of xorshift mixing
- **pipeline.md:24** — `process` signature documented as `fn process(&img, level, &ctx) -> Cow<DynamicImage>` but actual signature returns `Result<Cow<'a, DynamicImage>>`
- **pipeline.md:52** — `verify_image_bytes(bytes, mac_key) -> Option<bool>` documented as a method on the pipeline but actual code is a free function `pub fn verify_image_bytes(img_bytes: &[u8], mac_key: &[u8]) -> Option<bool>`
- **async-api.md:14** — `verify_image_bytes_async` return type documented as `Option<bool>` but actual code returns `Result<Option<bool>>`
- **async-api.md:10-13** — All async function return types documented without `Result` wrapper (e.g., `Cow<'static, DynamicImage>`) but actual code returns `Result<Cow<'static, DynamicImage>>` etc.

## Incomplete Modules

- **cli.md:35-37** — Documents stdin reading ("Reads from stdin when no input files specified") but actual CLI code calls `std::process::exit(1)` when `input_files.is_empty()` — stdin support does not exist
- **cli.md:49-53 (Verification Mode)** — Documents "Report protection details (level, seed, intensity)" and "Verify HMAC signature if key provided" but actual verification code does not perform HMAC verification via CLI — it only checks metadata seed extraction and stego extraction; the `mac_key` CLI arg is only used for protection, not verification

## Duplicate Content

- **overview.md:88** vs **util-image.md:19** vs **jpeg-stego-f5.md:68** — "Two XorShiftRng implementations" warning is repeated three times across three documents
- **overview.md:89** vs **types.md:65-81** — "Private fields with getters" pattern described in both overview and types doc
- **overview.md:90** vs **types.md:94-103** — `Arc<ProtectionConfig>` design described in both
- **overview.md:3-56 (Module Map)** vs **overview.md:60-81 (Component Deep Dives table)** — The module map and the deep-dive table list the same files/modules, creating redundant navigation
- **protected-noise.md:35-45** vs **protected-enhanced.md:17-31** — Enhanced protector wrapping `NoiseProtector::enhanced()` described in both files
- **util-image.md:42-48 (PerturbationRuntime)** vs **util-image.md:32-37 (PerturbationParams)** — Both describe overlapping pre-computed perturbation infrastructure

## Outdated Cross-References

- **overview.md:54** — Module map lists `src/jpeg_transcoder/mod.rs` as "Transcoder entry, scan data utilities" but actual `mod.rs` is primarily the `JpegTranscoder` struct and `scan_utils` module — description is incomplete
- **overview.md:38** — Module map lists `src/async_api.rs` as "Async wrappers (tokio)" but does not note it is behind `#[cfg(feature = "async")]`
- **constants.md:24** — Module interactions claim `SPLITMIX64_SEED` is referenced in `util/seed.rs` but `util/seed.rs` does not use this constant — it hardcodes the same value inline
- **constants.md:24** — Module interactions claim `STEGO_OFFSET_SEED_1` is referenced in `steganography.rs` only, but it is also used in `steganography.rs` for seed derivation
- **protected-metadata-trap.md:105** — Claims `STEGO_OFFSET_SEED_1` from `protected/constants.rs` is used for seed embedding, but `metadata_trap.rs` does not reference this constant
- **protected-noise.md:49** — Claims `NoiseProtector` is selected for `Standard` (10x) and `Enhanced` (12x via `EnhancedProtector`) in `lib.rs`, but `lib.rs` routes `Enhanced` through `MultiProtector::Enhanced` → `EnhancedProtector` → `NoiseProtector::enhanced()` — the noise protector itself is never directly called for Enhanced
- **cli.md:65** — Lists `rayon` as a dependency of the CLI, but `rayon` is used via the `cloakrs` library, not directly in CLI code (the CLI calls `process_image_bytes` which uses the library's parallel internals)
- **pipeline.md:3** — Claims `src/lib.rs` is "~745 lines" — actual is 745 lines (correct but worth noting line counts in docs may drift)
