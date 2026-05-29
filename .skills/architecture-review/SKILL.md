---
name: architecture-review
description: Use when reviewing, verifying, or updating architecture documentation against actual source code in the cloakrs codebase. Triggers on tasks like "verify architecture docs", "check doc accuracy", "review architecture documents", "find doc discrepancies", or when editing files in architecture/ directory.
---

# Architecture Documentation Review

Systematic workflow for verifying architecture documents against the cloakrs codebase.

## Quick Reference

- Architecture docs live in `architecture/` (21 files)
- Review outputs go to `plans/`
- Source code is in `src/`
- Use `rg` (ripgrep) for fast content search, `glob` for file patterns

## Review Workflow

### 1. Read the target architecture document completely

### 2. For every claim in the document, verify against source code:
- **Type names and fields**: Search for struct/enum definitions in `src/`
- **Function signatures**: Search for `fn ` patterns — check parameter types, return types, visibility
- **Constants**: Search for `const ` and `static ` — verify values and types
- **Module structure**: Compare module tree in docs against actual `src/` layout
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
| `Result<T>` returns | `Option<T>` returns | Various |
| `i64` array elements | `i16` array elements | `Coefficients` type |
| Wrong enum variants | Actual enum variants | `DmiValue`, `TranscoderError` |
| `String` fields | `Option<String>` fields | `Iscc.meta` |

### 4. Key source files to always check

- `src/types.rs` — All core type definitions, constructors, getters
- `src/traits.rs` — Protector trait, VariantLoader trait
- `src/lib.rs` — Pipeline orchestration, public API
- `src/error.rs` — Error variants
- `src/protected/steganography.rs` — Stego payload format, extraction, verification
- `src/jpeg_transcoder/mod.rs` — Coefficients type, TranscoderError

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
- The JPEG transcoder has two separate PRNG implementations (`XorShiftRng` vs `F5XorShiftRng`) — never interchange
- `process_bytes` skips dimension validation that `process` enforces
- ISCC implementation is NOT standard-compliant — uses custom component codes
