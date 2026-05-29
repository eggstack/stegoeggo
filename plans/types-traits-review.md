# Types and Traits Architecture Review

**Reviewed:** `architecture/types.md`, `architecture/traits.md` vs `src/types.rs`, `src/traits.rs`

---

## Verified Claims

### types.rs
- **ProtectionLevel enum** (lines 54-64): Five variants (Disabled, Light, Standard, Enhanced, Strong), `#[default]` on Standard — matches docs
- **`to_byte()` / `from_byte()`** (lines 77-96): Correctly maps 0-4 to variants
- **ImageOutputFormat enum** (lines 99-107): Png, Jpeg, WebP with `#[default]` on Png — matches docs
- **`from_magic_bytes()`** (lines 122-136): PNG `[0x89, 0x50, 0x4E, 0x47]`, JPEG `[0xFF, 0xD8, 0xFF]`, WebP `RIFF....WEBP` — matches docs
- **`extension()`** (lines 151-157): Returns `"png"`, `"jpg"`, `"webp"` — matches docs (note: returns `"jpg"` not `"jpeg"`)
- **ProtectionContext builder pattern** (lines 360-607): All `with_*` methods return `Self` with `#[must_use]` — matches docs
- **Intensity clamping** (line 366): `intensity.clamp(0.0, 1.0)` — matches docs
- **Default seed warning** (lines 332-338): Comment warns about predictability — matches docs
- **Private fields with getters** (lines 297-606): All fields private, public getters — matches docs
- **`inject_metadata` three-state semantics** (lines 304-313, 567-573): `None` = level default, `Some(true)` = force on, `Some(false)` = force off — matches docs exactly
- **`inject_legal_claims` semantics** (lines 315-323, 577-582): `None` = never inject, `Some(true)` = force on, `Some(false)` = force off — matches docs
- **`#[serde(skip)]` on `config`** (line 328): Present and documented — matches docs
- **ProtectedVariant::cache_key()** (lines 647-657): Returns `{hash}_{level}_{intensity}` with 4-decimal intensity rounding — matches docs
- **ProtectionConfig with `mac_key` and `legal_metadata`** (lines 261-272): Both `Option` fields, no builder pattern shown in docs (actual code has `with_mac_key` and `with_legal_metadata` methods) — partial match

### traits.rs
- **Protector trait definition** (lines 20-100): All 7 methods present (`apply`, `apply_bytes`, `name`, `protection_level`, `estimated_latency_ms`, `modifies_pixels`, `is_enabled`) — matches docs
- **`apply` returns `Cow<DynamicImage>`** (lines 23-27): True, with `#[must_use]` on builder methods — matches
- **`modifies_pixels` default `true`** (lines 97-99): Correct — matches
- **NoOpLoader** (lines 117-127): `load_variant` returns `Ok(None)`, `store_variant` returns `Ok(())` — matches docs

---

## Discrepancies

### 1. StegoPayload location (types.md:131 vs actual)
- **Doc**: Describes StegoPayload in `architecture/types.md` as if it's in `src/types.rs`
- **Actual**: StegoPayload is defined in `src/protected/steganography.rs:1029-1056`
- **Impact**: Low — it's exported from lib.rs and publicly accessible, but the source location in docs is wrong

### 2. DmiValue auto-mapping claim (types.md:55)
- **Doc claims**: "Auto-mapped from ProtectionLevel: Light→Prohibited, Standard→ProhibitedAiMlTraining, Enhanced→ProhibitedGenAiMlTraining, Strong→Prohibited"
- **Actual**: No `impl From<ProtectionLevel> for DmiValue` exists. The mapping exists only as a local helper in `metadata_trap.rs:104-107`:
  ```rust
  ProtectionLevel::Light => Some(DmiValue::Prohibited),
  ProtectionLevel::Standard => Some(DmiValue::ProhibitedAiMlTraining),
  // etc.
  ```
- **Impact**: Medium — someone reading types.md might expect `DmiValue::from(level)` to work, but it doesn't

### 3. LegalMetadata field name (types.md:111 vs types.rs:177)
- **Doc**: Lists field as `ai_training_constraints`
- **Actual**: Field is named `ai_constraints` (types.rs:177)
- **Getter**: `ai_constraints()` at types.rs:206 (doc says `ai_training_constraints()`)
- **Impact**: Low — getter method name follows actual field name

### 4. ProtectionContext method name (types.md:92 vs types.rs:435)
- **Doc example**: `ctx.with_output_format(ImageOutputFormat::Jpeg)`
- **Actual**: Method is `with_format()` not `with_output_format()` (types.rs:435)
- **Impact**: Medium — documentation examples won't compile as shown

### 5. ProtectedVariant field (types.md:118-126 vs types.rs:612-620)
- **Doc**: Lists 6 fields: `variant_id`, `original_hash`, `perturbation_data`, `intensity`, `width`, `height`
- **Actual**: Has 7 fields — includes `protection_level: ProtectionLevel` at types.rs:615
- **Impact**: Low — extra field is fine, just missing from docs

### 6. ProtectedVariant::new signature (types.md:129 vs types.rs:627-643)
- **Doc**: `new(hash, level, perturbation_data, intensity, width, height)` with note "No target model parameter"
- **Actual**: `new(original_hash, protection_level, perturbation_data, intensity, width, height)` — same signature but `protection_level` IS included as first positional arg after hash
- **Impact**: The docs say "No target model parameter" which is accurate (there's no target model), but the signature shows `level` which IS present

---

## Bugs Found

### 1. PassthroughProtector estimated_latency_ms (traits.rs:42-44)
- **Code**: `fn estimated_latency_ms(&self) -> u32 { 0 }`
- **Doc claim** (traits.md:35): `estimated_latency_ms = 0`
- **Status**: These actually match — the discrepancy I noted earlier was incorrect. traits.md:35 correctly shows latency of 0.
- **Correction**: No bug here; this is correctly documented.

### 2. StegoPayload version() returns u8, not u32
- **Doc** (types.md:138): `version() -> u8` (says u8)
- **Actual** (steganography.rs:1053): `pub fn version(&self) -> u8` — matches
- **Status**: Verified correct.

### 3. No bugs found in trait method signatures or core types

---

## Improvement Opportunities

### 1. Missing From<ProtectionLevel> for DmiValue
- **Suggestion**: Add an `impl From<ProtectionLevel> for DmiValue` in types.rs to match the documented auto-mapping behavior
- **Location**: types.rs around line 52 (after DmiValue impl block)
- **Rationale**: Would make the documented mapping actually usable and match the pattern described in docs

### 2. Rename LegalMetadata field for clarity
- **Suggestion**: Rename `ai_constraints` to `ai_training_constraints` in types.rs:177 to match documentation
- **Alternative**: Update docs to match code (less breaking change for consumers)
- **Rationale**: The field name in docs is more descriptive; consistency helps users reading both

### 3. Fix documentation example
- **Suggestion**: Change `with_output_format` to `with_format` in architecture/types.md:92
- **Location**: types.md:92
- **Rationale**: Current example won't compile

### 4. Add protection_level to ProtectedVariant documentation
- **Suggestion**: Add `protection_level` to the table in types.md:118-126
- **Location**: architecture/types.md around line 124
- **Rationale**: Documentation should reflect all public fields

### 5. StegoPayload documentation location
- **Suggestion**: Either move StegoPayload reference to a steganography-specific doc, or note that its implementation lives in `src/protected/steganography.rs`
- **Location**: architecture/types.md:131-138
- **Rationale**: Source location is currently misleading

---

## Stale References

### types.md
| Reference | Actual | Location |
|-----------|--------|----------|
| `StegoPayload` defined in types.rs | Actually in `steganography.rs:1029` | types.md:131 |
| `with_output_format()` | `with_format()` | types.md:92 |
| `ai_training_constraints` field/getter | `ai_constraints` | types.md:111, types.rs:177,206 |
| `protection_level` missing from ProtectedVariant docs | Field exists at types.rs:615 | types.md:118-126 |

### traits.md
No stale references found. All method signatures and implementations match correctly.

---

## Summary

| Category | Count |
|----------|-------|
| Verified Claims | 17 |
| Discrepancies | 6 |
| Bugs Found | 0 |
| Improvement Opportunities | 5 |
| Stale References | 4 |

**Overall Assessment**: The documentation is largely accurate with the exception of StegoPayload's location and a few method/field name mismatches. The core architectural patterns (builder, getters, three-state options, serde skip) are all correctly documented. No logic errors or edge-case panics were identified in the reviewed code.