---
name: architecture-review
description: Use when reviewing, verifying, or updating architecture documentation against actual source code in the stegoeggo codebase. Triggers on tasks like "verify architecture docs", "check doc accuracy", "review architecture documents", "find doc discrepancies", or when editing files in architecture/ directory.
---

# Architecture Documentation Review

Systematic workflow for verifying architecture documents against the stegoeggo codebase.

## Quick Reference

- Architecture docs live in `architecture/` (30 files)
- Review outputs go to `plans/`
- Source code is in `src/` (root crate) and `stegoeggo-stego/src/` (carrier crate)
- Use `rg` (ripgrep) for fast content search, `glob` for file patterns

## Review Workflow

### 1. Read the target architecture document completely

### 2. For every claim in the document, verify against source code:
- **Type names and fields**: Search for struct/enum definitions in `src/` and `stegoeggo-stego/src/`
- **Function signatures**: Search for `fn ` patterns — check parameter types, return types, visibility
- **Constants**: Search for `const ` and `static ` — verify values and types
- **Module structure**: Compare module tree in docs against actual layout
- **Behavioral claims**: Read the implementation to verify described behavior
- **Return types**: Check for `Result`, `Option`, `Cow` wrappers that docs may omit
- **Visibility**: Check `pub`, `pub(crate)`, private — docs often get this wrong

### 3. Common discrepancy patterns in this codebase

| What docs say | What code actually has | Files affected |
|---|---|---|
| `f64` return types | `u32` return types | `estimated_latency_ms` everywhere |
| Free functions | `&self` methods | steganography extract/verify functions |
| `Vec<u8>` fields | `Option<Vec<u8>>` fields | `ProtectionConfig.mac_key` |
| `bool` fields | `Option<bool>` fields | `inject_metadata`, `inject_legal_claims` |
| Public methods | Private methods | `assemble_jpeg`, `get_scan_data_start` |
| `Result<T>` returns | Direct returns | `verify_image_bytes` returns `VerificationStatus` directly |
| `i64` array elements | `i16` array elements | `Coefficients` type |
| Wrong enum variants | Actual enum variants | `DmiValue`, `TranscoderError` |
| `String` fields | `Option<String>` fields | `Iscc.meta` |
| V2 as current payload | V3 is the default | `protected/steganography/`, `payload_v3/` |
| 17 Error variants | 19 Error variants (18 always-available + 1 async) | `error.rs` |
| 7 ProtectionWarning variants | 8 ProtectionWarning variants | `types.rs` |
| `Option<bool>` returns | `VerificationStatus` returns | `verify_payload_from_bytes_with_key` |
| `src/protected/steganography.rs` | Split into 5 modules under `src/protected/steganography/` | `marker.rs`, `embed.rs`, `extract.rs`, `verify.rs`, `legacy.rs` |

### 4. Key source files to always check

- `src/types.rs` — All core type definitions, constructors, getters (~5100 lines)
- `src/traits.rs` — Protector trait
- `src/lib.rs` — Pipeline orchestration, public API, module declarations (~2244 lines)
- `src/error.rs` — Error variants (19 total: 18 always-available + 1 async-only `Task`)
- `src/protected/steganography/mod.rs` — Facade + shared types; algorithm modules are `marker.rs`, `embed.rs`, `extract.rs`, `verify.rs`, `legacy.rs`
- `stegoeggo-stego/src/jpeg_transcoder/` — JPEG DCT internals (private to carrier)
- `src/payload_v3/types.rs` — V3 payload constants and types
- `stegoeggo-stego/src/constants.rs` — Carrier-level tuning constants (`STEGO_SPREAD_FACTOR`, `STEGO_OFFSET_SEED_1`, `SPLITMIX64_SEED`, `MIN_REDUNDANCY`, `MAX_REDUNDANCY`)
- `src/protected/constants.rs` — Application-level constants (`STEGO_OFFSET_SEED_1`, `XORSHIFT_SEED_OFFSET`)

### 5. Document findings in this format

```markdown
## Document: [name].md
### Verified Claims
- [claim] — **Confirmed** (`file:line`)

### Discrepancies
1. **[What's wrong]** — Doc says [X] but code has [Y] (`file:line`)

### Potential Bugs/Edge Cases
- [issue description]
```

## Known Gotchas

- `ProtectionContext` fields are all private with getter methods — docs often show public fields
- `Cow<'a, DynamicImage>` returns require lifetime annotations that docs frequently omit
- `Option<bool>` fields have ambiguous `None` vs `false` semantics — document this explicitly
- The carrier crate has two separate PRNG implementations (`PixelSelectionRng` in `util/image.rs` and `DctCoefficientRng` in `stegoeggo-stego/src/jpeg_transcoder/stego_f5.rs`) — never interchange
- ISCC implementation is NOT standard-compliant — uses custom component codes
- `src/constants.rs` does NOT exist as a top-level file — constants are in `src/protected/constants.rs` (application) and `stegoeggo-stego/src/constants.rs` (carrier)
- The JPEG transcoder lives in the carrier crate (`stegoeggo-stego/src/jpeg_transcoder/`), not the root crate
- `verify_image_bytes` returns `VerificationStatus` directly, not `Result<VerificationStatus>`

## Verified Discrepancies (do not re-report these)

These have been fixed in documentation — if the code hasn't changed, these are now correctly documented:

- **`XorShiftRng::new`** uses `wrapping_add`, not XOR — use `seed.wrapping_add(XORSHIFT_SEED_OFFSET)`
- **`parallel_threshold()`** scales as `cores * 64 * 64` — 1c:4096, 4c:16384, 16c:65536
- **`verify_image_bytes`** DOES perform DCT stego verification — contrary to old docs
- **CLI batch** does NOT preserve directory structure — outputs flat to `-o` dir
- **`LegalMetadata`** field is `ai_constraints` (not `ai_training_constraints`)
- **`ProtectionContext::with_format()`** (not `with_output_format()`)
- **DmiValue mapping** is via `ProtectionLevel::default_policy()` in `types.rs` — no `impl From<ProtectionLevel> for DmiValue`
- **Error enum** has 19 variants (18 always-available + 1 async-only `Task`) — 5 structured variants (`InputTooLarge`, `DimensionsExceeded`, `ContainerLimitExceeded`, `MetadataLimitExceeded`, `VerificationBudgetExceeded`) were added for resource limits
- **`ProtectionWarning`** has 8 variants — `ContradictoryLegalClaims` and `MissingRightsConstraints` were added
- **`ExecutionReport`** has 9 fields — `authentication_performed` does not exist; replaced by `effective_policy`, `effective_dmi`, `stego_attempted`, `format_transcoded`, `resource_usage`, `embed_summary`
- **`LegalMetadata`** has 16 fields — 8 additional fields: `usage_terms_lang`, `credit_line`, `copyright_owner`, `licensor_name`, `licensor_email`, `licensor_url`, `metadata_date`, `notice_applied_at`
- **Tile size** clamp is `32..=1024` (0 disables), not `>= 16`
- **`ed25519-dalek`** version is `3`, not `2`
- **V3 is current** — `V3_PAYLOAD_VERSION = 3` is the default; V2/V1 are extraction-only legacy
- **`CURRENT_PAYLOAD_VERSION`** does not exist — the constant is `V3_PAYLOAD_VERSION` in `src/payload_v3/types.rs`
- **`EvidenceStrength`** has 4 variants: `NoNoticeFound`, `MetadataNoticeOnly`, `MetadataNoticeAndBestEffortStego`, `MetadataNoticeAndAuthenticatedProvenance`
- **`TrustEvaluation`** does not exist as a struct — verification uses `NoticeVerification` (26 fields) and `VerificationResult`
- **Steganography adapter** is split into 5 modules: `marker.rs`, `embed.rs`, `extract.rs`, `verify.rs`, `legacy.rs` behind `SteganographyProtector` facade
- **Generic carrier crate** public API: `lsb`, `jpeg`, `frame`, `error`, `types` modules; `jpeg_transcoder` and `lsb_internal` are `pub(crate)`
