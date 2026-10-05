---
name: architecture-review
description: Use when reviewing, verifying, or updating architecture documentation against actual source code in the stegoeggo codebase. Triggers on tasks like "verify architecture docs", "check doc accuracy", "review architecture documents", "find doc discrepancies", or when editing files in architecture/ directory.
---

# Architecture Documentation Review

Systematic workflow for verifying architecture documents against the stegoeggo codebase.

## Quick Reference

- Architecture docs live in `architecture/` (42 files: `overview.md` + `review_plan.md` (historical, gitignored) + 40 component deep-dives)
- Review outputs go to `plans/`
- Source code is in `src/` (root crate), `stegoeggo-stego/src/` (carrier crate), `stegoeggo-cli/src/` (CLI), and `bindings/{c,node,python}/src/` (FFI leaves, each a nested workspace outside the main one)
- Use `rg` (ripgrep) for fast content search, `glob` for file patterns
- If the work touches an FFI surface, also load `.skills/bindings/SKILL.md`

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
| 17 Error variants | 21 Error variants (20 always-available + 1 async-only `Task`) | `error.rs` |
| 7 ProtectionWarning variants | 8 ProtectionWarning variants | `types.rs` |
| `Option<bool>` returns | `VerificationStatus` returns | `verify_payload_from_bytes_with_key` |
| `src/protected/steganography.rs` | Split into 5 modules under `src/protected/steganography/` | `marker.rs`, `embed.rs`, `extract.rs`, `verify.rs`, `legacy.rs` |
| "all CI except `ci.yml` is scheduled" | 4 workflows also run on push/PR (path-filtered) | `c-binding.yml`, `node-binding.yml`, `python-binding.yml`, `release-drift.yml` |
| "3 or 4 crates in the workspace" | 4 members + 3 **excluded** binding crates | root `Cargo.toml` `exclude = ["bindings"]` |
| `metadata_trap/{notice,png,jpeg,webp,common}.rs` | 6 modules — `spec.rs` also exists | `src/protected/metadata_trap/` |
| "examples must keep compiling" (some are unused) | All 4 are Cargo-registered example targets | `cargo metadata` shows 4 examples |

### 4. Key source files to always check

- `src/types/` — Core type definitions by domain behind `src/types.rs` re-exports: `rights.rs`, `compat.rs`, `legal.rs`, `context.rs`, `verification.rs`, `warnings.rs`, `request.rs`
- `src/traits.rs` — Protector trait
- `src/lib.rs` — Public API, module declarations, container-accounting walker (canonical executors live in `src/pipeline.rs`)
- `src/error.rs` — Error variants (21 total: 20 always-available + 1 async-only `Task`)
- `src/verification/report.rs` — `VerificationReport`, `TrustEvaluation`, sub-verification types
- `src/protected/steganography/mod.rs` — Facade + shared types; algorithm modules are `marker.rs`, `embed.rs`, `extract.rs`, `verify.rs`, `legacy.rs`
- `src/protected/metadata_trap.rs` — Facade; format modules are `metadata_trap/notice.rs`, `spec.rs`, `png.rs`, `jpeg.rs`, `webp.rs`, `common.rs`
- `src/pipeline.rs` — Canonical plan executors (`process_plan_bytes`, `execute_*`, `warnings_from_embed_outcome`)
- `stegoeggo-stego/src/jpeg_transcoder/` — JPEG DCT internals (private to carrier)
- `src/payload_v3/types.rs` — V3 payload constants and types
- `stegoeggo-stego/src/constants.rs` — Carrier-level tuning constants (`STEGO_SPREAD_FACTOR`, `STEGO_OFFSET_SEED_1`, `SPLITMIX64_SEED`, `MIN_REDUNDANCY`, `MAX_REDUNDANCY`)
- `src/protected/constants.rs` — Application-level constants (`STEGO_OFFSET_SEED_1`)

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
- The retired root-crate pixel RNG was removed; `DctCoefficientRng` in `stegoeggo-stego/src/jpeg_transcoder/stego_f5.rs` is the only stego PRNG
- ISCC implementation is NOT standard-compliant — uses custom component codes
- `src/constants.rs` does NOT exist as a top-level file — constants are in `src/protected/constants.rs` (application) and `stegoeggo-stego/src/constants.rs` (carrier)
- The JPEG transcoder lives in the carrier crate (`stegoeggo-stego/src/jpeg_transcoder/`), not the root crate
- `verify_image_bytes` returns `VerificationStatus` directly, not `Result<VerificationStatus>`

## Verified Discrepancies (do not re-report these)

These have been fixed in documentation — if the code hasn't changed, these are now correctly documented:

- **`parallel_threshold()`** and **`LazyLock` singletons** do NOT exist anywhere in the current codebase — docs claiming them are stale; free functions delegate directly to `request_from_legacy()` + `process_request_bytes()`
- **`verify_image_bytes`** DOES perform DCT stego verification — contrary to old docs
- **CLI batch** does NOT preserve directory structure — outputs flat to `-o` dir
- **`LegalMetadata`** field is `ai_constraints` (not `ai_training_constraints`)
- **`ProtectionContext::with_format()`** (not `with_output_format()`)
- **DmiValue mapping** is via `ProtectionLevel::default_policy()` in `types.rs` — no `impl From<ProtectionLevel> for DmiValue`
- **Error enum** has 21 variants (20 always-available + 1 async-only `Task`) — 5 structured resource-limit variants (`InputTooLarge`, `DimensionsExceeded`, `ContainerLimitExceeded`, `MetadataLimitExceeded`, `VerificationBudgetExceeded`) plus structured `InsufficientCapacity` were added after the original 14, and `ResourceLimitExceeded(String)` carries the `StegoError::ResourceLimitExceeded` carrier conversion
- **`ProtectionWarning`** has 8 variants — `ContradictoryLegalClaims` and `MissingRightsConstraints` were added
- **`ExecutionReport`** has 9 fields — `authentication_performed` does not exist; replaced by `effective_policy`, `effective_dmi`, `stego_attempted`, `format_transcoded`, `resource_usage`, `embed_summary`
- **`LegalMetadata`** has 16 fields — 8 additional fields: `usage_terms_lang`, `credit_line`, `copyright_owner`, `licensor_name`, `licensor_email`, `licensor_url`, `metadata_date`, `notice_applied_at`
- **Tile size** clamp is `32..=1024` (0 disables), not `>= 16`
- **`ed25519-dalek`** version is `3`, not `2`
- **V3 is current** — `V3_PAYLOAD_VERSION = 3` is the default; V2/V1 are extraction-only legacy
- **`CURRENT_PAYLOAD_VERSION`** does not exist — the constant is `V3_PAYLOAD_VERSION` in `src/payload_v3/types.rs`
- **`EvidenceStrength`** has 4 variants: `NoNoticeFound`, `MetadataNoticeOnly`, `MetadataNoticeAndBestEffortStego`, `MetadataNoticeAndAuthenticatedProvenance`
- **Verification types are all real** — do not flag these as fabricated: `NoticeVerification` (`src/types/verification.rs`, notice-level evidence), `VerificationResult` (`src/types/verification.rs`, enum), `VerificationReport` + `TrustEvaluation` + `TrustEvaluationBuilder` (`src/verification/report.rs`). `architecture/verification.md` documents them correctly
- **`VerificationStatus` is live** — not deprecated; it is the return type of `verify_image_bytes`. Do not mark it deprecated or suggest migrating away from it
- **Steganography adapter** is split into 5 modules: `marker.rs`, `embed.rs`, `extract.rs`, `verify.rs`, `legacy.rs` behind `SteganographyProtector` facade
- **Generic carrier crate** public API: `constants`, `error`, `frame`, `jpeg`, `limits`, `lsb`, `pixels`, `prepared`, `types`, and `webp` (feature `webp`) modules; `jpeg_transcoder` and `lsb_internal` are `pub(crate)`; `application_support` is `pub` behind the `application-support` feature but `#[doc(hidden)]`. `limits.rs` and `webp.rs` are `pub mod` — do not report them as private or undocumented
- **`bindings/` is EXCLUDED from the Cargo workspace** (`exclude = ["bindings"]` in the root `Cargo.toml`). Three leaf crates — `bindings/c` (cdylib + committed cbindgen header, ABI-V1, 90-symbol export manifest), `bindings/node` (napi-rs), `bindings/python` (PyO3 + maturin) — each a nested `[workspace]`, `publish = false`, pinning `stegoeggo = "=0.4.2"` with `default-features = false`, and each forcing `panic = "unwind"` in dev and release so panics can be caught at the FFI boundary. They are covered by `architecture/bindings-c.md`, `bindings-node.md`, `bindings-python.md`, and validated by path-filtered CI (`c-binding.yml`, `node-binding.yml`, `python-binding.yml`, all on push **and** PR to `main`) that is NOT part of `check.sh`
- **12 CI workflows exist**, not 5: `ci`, `assurance`, `external-verification`, `fuzz`, `release-binaries`, `release-drift`, `c-binding`, `node-binding`, `python-binding`, `release-c`, `release-node`, `release-python`
