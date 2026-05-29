# Core Framework Review Findings

## Document: overview.md

### Verified Claims
- Module tree matches actual `src/` structure (lib.rs, types.rs, traits.rs, error.rs, async_api.rs, protected/*, jpeg_transcoder/*, util/*)
- `pub(crate)` visibility on jpeg_transcoder, protected, and util modules confirmed in src/lib.rs:100-102
- Strategy pattern with Protector trait confirmed
- Cow returns for Protector::apply confirmed (trait signature returns `Cow<'a, DynamicImage>`)
- JPEG fast path exists (apply_multi_protector_bytes in lib.rs:346)
- Two XorShiftRng implementations noted in AGENTS.md (not verifiable from these files alone)
- Private fields with getters on ProtectionContext, ProtectedVariant, StegoPayload confirmed
- Arc wrapping for ProtectionConfig confirmed (types.rs:310)
- LazyLock singleton for DEFAULT_PIPELINE confirmed (lib.rs:144)
- Dependencies listed match Cargo.toml (image, jpeg-encoder, rayon, sha2/hmac, serde, subtle, tokio, clap, crc32fast)

### Discrepancies
- DmiValue variants listed as `ProhibitedScraping` and `ProhibitedAnyProcessing` in overview.md do NOT exist in code. Actual variants (types.rs:14-16): `ProhibitedExceptSearchEngineIndexing` and `ProhibitedSeeConstraints`
- High-level flow diagram shows Light → MetadataTrapProtector (metadata only, no pixel change) which is correct, but the description omits that Light also encodes to bytes and decodes back (apply_light_bytes in lib.rs:290-303), which can alter format/quality

### Improvement Opportunities
- The flow diagram could note that Light level encodes and re-decodes the image, which may subtly alter pixel values depending on format

### Potential Bugs/Edge Cases
- The DmiValue variant naming mismatch between docs and code could confuse users who read the docs and try to use `DmiValue::ProhibitedScraping` or `DmiValue::ProhibitedAnyProcessing`

---

## Document: pipeline.md

### Verified Claims
- ProtectionPipeline struct fields (passthrough, metadata_trap, noise, enhanced, precomputed, steganography) confirmed — though field order differs from doc listing (doc lists passthrough first, noise second, metadata_trap fifth; code has passthrough, metadata_trap, noise...)
- `process` method takes `&DynamicImage` and returns `Result<Cow<'a, DynamicImage>>` confirmed (lib.rs:211-216)
- `process_bytes` takes `&[u8]` and returns `Result<Vec<u8>>` confirmed (lib.rs:318-344)
- `register_precomputed_variants` delegates to PrecomputedProtector confirmed (lib.rs:309-311)
- Pipeline flow (perturbation → stego → encode → metadata injection) confirmed in apply_protector_pipeline (lib.rs:255-285)
- JPEG fast path (encode → DCT stego → metadata) confirmed (lib.rs:265-274)
- Non-JPEG path (pixel stego → encode → metadata) confirmed (lib.rs:276-284)
- Format routing from magic bytes confirmed via ImageOutputFormat::from_magic_bytes (lib.rs:354)
- Error::InvalidFormat returned when format cannot be determined confirmed (lib.rs:355)
- LazyLock singleton usage for convenience functions confirmed (lib.rs:144)

### Discrepancies
- Pipeline flow doc says Standard/Enhanced/Strong follows "1. Apply perturbation → 2. If JPEG output: encode → DCT stego → metadata" but code (lib.rs:364-369) shows the JPEG fast path in apply_multi_protector_bytes only applies when BOTH input AND output are JPEG. When input is non-JPEG and output is JPEG, it goes through apply_protector_pipeline which still does the JPEG encode→stego→metadata path. The doc is accurate but could be clearer about the dual-path nature
- `process_image_bytes` convenience function (lib.rs:439-456) has additional logic not mentioned in pipeline.md: it performs format detection and sets input_format on the context before calling process_bytes. The doc doesn't mention this auto-detection step

### Improvement Opportunities
- The `process_image_bytes` convenience function's auto-detection behavior (setting input_format from magic bytes) is undocumented in pipeline.md
- The dimension validation only applies to the `process` method (pixel path), not to `process_bytes` — this asymmetry is not documented

### Potential Bugs/Edge Cases
- `process_bytes` (lib.rs:318-344) does NOT validate dimensions, while `process` (lib.rs:217) does. Large images processed via byte path bypass max_dimension checks
- In `apply_multi_protector_bytes`, when input is JPEG and output is non-JPEG (e.g., JPEG→PNG conversion), the code takes the "non-JPEG-in" path (lib.rs:372) which decodes to pixels and applies full pipeline including noise perturbation. This is correct behavior but the fast path is only triggered for JPEG→JPEG

---

## Document: traits.md

### Verified Claims
- Protector trait is `pub trait Protector: Send + Sync` confirmed (traits.rs:20)
- `apply` signature `fn apply<'a>(&self, img: &'a DynamicImage, ctx: &ProtectionContext) -> Result<Cow<'a, DynamicImage>>` confirmed (traits.rs:23-27)
- `apply_bytes` default implementation decodes → apply → re-encodes confirmed (traits.rs:33-79)
- `name` returns `&'static str` confirmed (traits.rs:82)
- `protection_level` returns `ProtectionLevel` confirmed (traits.rs:85)
- `is_enabled` returns `bool` with default `true` confirmed (traits.rs:91-93)
- `modifies_pixels` returns `bool` with default `true` confirmed (traits.rs:97-99)
- VariantLoader trait with `load_variant` and `store_variant` confirmed (traits.rs:106-112)
- NoOpLoader returns None and Ok(()) confirmed (traits.rs:119-127)
- Protector level assignments correct (Passthrough→Disabled, MetadataTrap→Light, Noise→Standard, Enhanced→Enhanced, Precomputed→Strong, Steganography→Standard)

### Discrepancies
- `estimated_latency_ms` return type: doc says `f64` (traits.md:15) but actual code returns `u32` (traits.rs:88). All implementations also return `u32`
- `VariantLoader::load_variant` return type: doc says `Option<ProtectedVariant>` (traits.md:48) but actual code returns `Result<Option<ProtectedVariant>>` (traits.rs:108)
- `VariantLoader::store_variant` return type: doc says `Result<()>` (traits.md:49) which matches code (traits.rs:111)
- SteganographyProtector estimated_latency_ms: doc says "~3.0" (traits.md:40) but actual code returns `2` (steganography.rs:1019-1021)
- MetadataTrapProtector estimated_latency_ms: doc says "~1.0" (traits.md:37) but actual code returns `2` (metadata_trap.rs:564-566)
- NoiseProtector estimated_latency_ms: doc says "~5.0" (traits.md:38) but actual code returns `3` (noise.rs:83-85)
- EnhancedProtector estimated_latency_ms: doc says "~7.0" (traits.md:39) but actual code returns `5` (enhanced.rs:49-51)

### Improvement Opportunities
- The `estimated_latency_ms` values in the implementations table are significantly off from actual values. This table should be updated to reflect the real values
- The `apply_bytes` default implementation behavior when `modifies_pixels()` returns false (returns input bytes unchanged) is documented but could be more explicit about the early return path

### Potential Bugs/Edge Cases
- None found beyond the documentation discrepancies

---

## Document: types.md

### Verified Claims
- ProtectionLevel enum with five variants confirmed (types.rs:57-64)
- `to_byte()` / `from_byte()` methods confirmed (types.rs:77-96)
- Default returns Standard confirmed (types.rs:60)
- ImageOutputFormat enum with Png, Jpeg, WebP confirmed (types.rs:102-107)
- `from_magic_bytes` PNG detection `[0x89, P, N, G]` confirmed (types.rs:126)
- `from_magic_bytes` JPEG detection `[0xFF, 0xD8, 0xFF]` confirmed (types.rs:129)
- `from_magic_bytes` WebP detection `RIFF....WEBP` confirmed (types.rs:132-134)
- `from_extension` method confirmed (types.rs:113-119)
- `extension()` returns "png", "jpg", "webp" confirmed (types.rs:151-157)
- `to_image_format()` conversion confirmed (types.rs:159-165)
- DmiValue auto-mapping from ProtectionLevel confirmed (metadata_trap.rs:104-107): Light→Prohibited, Standard→ProhibitedAiMlTraining, Enhanced→ProhibitedGenAiMlTraining, Strong→Prohibited
- ProtectionContext::new(intensity, seed) with intensity clamped to [0.0, 1.0] confirmed (types.rs:345-347)
- ProtectionContext fields are all private with getter methods confirmed (types.rs:296-311, getters at 488-556)
- Builder methods with #[must_use] confirmed (types.rs:364, 372, 385, 407, 415, 423, 430, 437, 445, 452, 459, 467, 474, 482)
- ProtectionConfig with mac_key and legal_metadata confirmed (types.rs:261-272)
- ProtectionContext.config is #[serde(skip)] confirmed (types.rs:309)
- ProtectedVariant::new signature confirmed (types.rs:576-593) with parameters (original_hash, protection_level, perturbation_data, intensity, width, height)
- StegoPayload getters (protection_level, seed, intensity, version) confirmed (steganography.rs:1036-1055)
- LegalMetadata builder methods with #[must_use] confirmed (types.rs:214-254)

### Discrepancies
- DmiValue variants listed as `ProhibitedScraping` and `ProhibitedAnyProcessing` in types.md do NOT exist in code. Actual variants: `ProhibitedExceptSearchEngineIndexing` and `ProhibitedSeeConstraints` (types.rs:14-16)
- `ProtectionConfig.mac_key` field: doc says `pub mac_key: Vec<u8>` (types.md:100) but actual code has `pub mac_key: Option<Vec<u8>>` (types.rs:269). The field is `Option<Vec<u8>>`, not plain `Vec<u8>`
- `ProtectionContext.inject_metadata` field: doc says `bool` default `true` (types.md:76) but actual code has `Option<bool>` default `None` (types.rs:304, 354). The `inject_metadata()` getter also returns `Option<bool>` not `bool` (types.rs:524-526)
- `ProtectionContext.inject_legal_claims` field: doc says `bool` default `false` (types.md:77) but actual code has `Option<bool>` default `None` (types.rs:305, 355). The getter returns `Option<bool>` (types.rs:529-531)
- `ProtectionContext.stego_redundancy` field: doc says `u8` (types.md:78) but actual code has `usize` (types.rs:306). The `with_stego_redundancy` method also takes `usize` (types.rs:468)
- `ProtectedVariant` field name: doc says `uuid: Uuid` (types.md:117) but actual code has `variant_id: uuid::Uuid` (types.rs:562)
- `ProtectedVariant::cache_key()` format: doc says returns `{uuid}_{hash}_{intensity}` (types.md:126) but actual code returns `{hash}_{level}_{intensity}` (types.rs:596-606). The UUID is not included; protection level string is used instead
- `StegoPayload::protection_level()` return type: doc says `ProtectionLevel` (types.md:133) but actual code returns `u8` (steganography.rs:1038)
- `ProtectedVariant::new()` parameter description: doc says "No target model parameter" (types.md:127) — the `TargetModel` concept was removed per AGENTS.md, so this note is accurate but the parameter list in the doc doesn't mention `protection_level` parameter name

### Improvement Opportunities
- The ProtectionContext fields table needs updating: `inject_metadata` and `inject_legal_claims` are `Option<bool>` with `None` defaults, not plain `bool`. The semantics of `None` vs explicit `true`/`false` should be documented (e.g., does `None` mean "use level default"?)
- The ProtectedVariant struct diagram should show `variant_id` not `uuid`, and the cache_key format should match actual code
- The StegoPayload documentation should note that `protection_level()` returns a `u8` byte value, not a `ProtectionLevel` enum

### Potential Bugs/Edge Cases
- The `inject_metadata: Option<bool>` ambiguity: callers cannot distinguish "not set" from "explicitly disabled" without understanding the `None` semantics. If `None` defaults to enabled for Standard+ levels, then `with_metadata_injection(false)` and not calling it at all would have different effects — this behavior is not documented

---

## Document: error.md

### Verified Claims
- `thiserror` usage confirmed (error.rs:8)
- Error enum with all listed variants confirmed (error.rs:12-67)
- `Io` variant wraps `std::io::Error` with `#[from]` confirmed (error.rs:20)
- `Serialization` wraps `serde_json::Error` with `#[from]` confirmed (error.rs:23)
- `Image` variant wraps `ImageError` with `#[from]` confirmed (error.rs:41)
- `#[cfg(feature = "async")]` on Task variant confirmed (error.rs:64)
- Result type alias confirmed (error.rs:69)
- All variants use String for simplicity confirmed (no lifetime parameters)
- `#[non_exhaustive]` on Error enum confirmed (error.rs:11)
- Variant descriptions match actual error messages in code

### Discrepancies
- `Io` variant: doc says "wraps `std::io::Error` directly" (error.md:63) — this is correct, uses `#[from]` for automatic conversion
- No discrepancies found in the error variants themselves

### Improvement Opportunities
- None found

### Potential Bugs/Edge Cases
- The `#[non_exhaustive]` attribute on Error means downstream crates cannot exhaustively match on Error variants. This is correct for library design but should be noted if any downstream code does `match` on Error

---

## Cross-Cutting Findings

1. **StegoPayload::protection_level() returns u8, not ProtectionLevel**: The types.md doc (types.md:133) claims this returns `ProtectionLevel`, but the actual implementation (steganography.rs:1038) returns `u8`. This is a significant type mismatch that could cause compilation errors for users following the docs.

2. **ProtectionConfig.mac_key is Option<Vec<u8>> not Vec<u8>**: The types.md doc (types.md:100) shows `pub mac_key: Vec<u8>` but the actual field is `Option<Vec<u8>>` (types.rs:269). Users following the doc would fail to compile.

3. **DmiValue variant names completely wrong in docs**: Both overview.md and types.md list `ProhibitedScraping` and `ProhibitedAnyProcessing` which don't exist. The actual variants are `ProhibitedExceptSearchEngineIndexing` and `ProhibitedSeeConstraints`. This would cause compilation failures for users following the docs.

4. **inject_metadata/inject_legal_claims Option<bool> semantics undocumented**: These fields are `Option<bool>` with `None` defaults in code, but documented as `bool` with `true`/`false` defaults. The behavior when `None` (presumably: use level default) vs explicit `false` (disable) is not documented anywhere.

5. **estimated_latency_ms values are approximations, not actuals**: All implementation values differ from the doc table. The doc uses "~" prefix suggesting approximation, but the actual values are specific integers (0, 2, 3, 5, 2, 2). The approximations are off by 40-100% in some cases.

6. **Dimension validation asymmetry**: `process()` validates dimensions (lib.rs:217) but `process_bytes()` does not (lib.rs:318). Images exceeding `max_dimension` can bypass the check when processed through the byte path.

7. **ProtectedVariant::cache_key() format mismatch**: Doc says `{uuid}_{hash}_{intensity}` but code generates `{hash}_{level}_{intensity}` with intensity rounded to 4 decimal places. The UUID is generated but not used in the cache key.

8. **process_image_bytes auto-detection**: The convenience function performs format detection from magic bytes and sets input_format on the context (lib.rs:444-453), which is not mentioned in pipeline.md. This is important because it affects whether the JPEG fast path is taken.
