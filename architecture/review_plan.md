# Architecture Documentation Review Plan

## Purpose

Systematically review all 21 architecture documents against the actual codebase to verify claims, identify discrepancies, surface bugs/improvements, and produce actionable improvement plans. Each module group is reviewed by a dedicated subagent that writes its findings to `plans/`.

## Scope

All files in `architecture/` **except** `review_plan.md` itself (this file). The 21 documents are organized into 4 module groups for parallel subagent review.

## Directory Layout

```
architecture/           # Source architecture documents (read-only during review)
plans/                  # Output: one improvement plan per module group
  core-framework.md     # Subagent 1 output
  protection-strategies.md  # Subagent 2 output
  jpeg-transcoder.md    # Subagent 3 output
  utilities-integration.md   # Subagent 4 output
```

---

## Phase 1: Module Group Reviews (Parallel)

Launch 4 subagents in parallel, one per module group. Each subagent:
1. Reads all assigned architecture documents completely
2. Reads all referenced source files to verify claims
3. Interrogates the code for bugs, edge cases, and improvement opportunities
4. Writes a structured improvement plan to `plans/<group>.md`

### Subagent 1: Core Framework (`plans/core-framework.md`)

**Documents:**
- `architecture/overview.md` — Architecture overview, module map, cross-cutting concerns
- `architecture/pipeline.md` — `ProtectionPipeline`, public API, format detection
- `architecture/traits.md` — `Protector` trait, `VariantLoader` trait
- `architecture/types.md` — `ProtectionLevel`, `ProtectionContext`, `ProtectedVariant`, `StegoPayload`, etc.
- `architecture/error.md` — Error enum, error handling strategy

**Source files to verify against:**
- `src/lib.rs` — Pipeline implementation, convenience functions
- `src/traits.rs` — Trait definitions
- `src/types.rs` — Type definitions, constructors
- `src/error.rs` — Error variants

**Review focus areas:**
- Verify module tree in overview.md matches actual `src/` structure
- Verify `ProtectionPipeline` flow description matches `src/lib.rs` implementation
- Verify trait method signatures and contracts match code
- Verify `ProtectedVariant::new()` constructor signature vs. doc (known discrepancy: doc shows `uuid` field but constructor doesn't take it)
- Verify `ProtectionConfig` field visibility (doc shows `pub` fields vs. private-fields convention)
- Verify format detection logic and `Error::InvalidFormat` usage
- Check for undocumented pipeline behavior (error propagation, identity paths, partial processing)
- Check for missing `#[must_use]` or incorrect builder patterns
- Interrogate: are there race conditions in pipeline orchestration? Are `Cow` return values used optimally?

**Output format for `plans/core-framework.md`:**
```markdown
# Core Framework Review Findings

## Document: overview.md
### Verified Claims
- [list claims confirmed in code]

### Discrepancies
- [doc vs. code mismatches]

### Improvement Opportunities
- [suggested documentation or code improvements]

### Potential Bugs/Edge Cases
- [anything concerning found during review]

## Document: pipeline.md
[same structure]

## Document: traits.md
[same structure]

## Document: types.md
[same structure]

## Document: error.md
[same structure]

## Cross-Cutting Findings
- [issues spanning multiple documents]
```

---

### Subagent 2: Protection Strategies (`plans/protection-strategies.md`)

**Documents:**
- `architecture/constants.md` — Tuning constants
- `architecture/protected-passthrough.md` — Disabled level no-op
- `architecture/protected-noise.md` — Standard adversarial noise
- `architecture/protected-enhanced.md` — Enhanced noise (12x multiplier)
- `architecture/protected-metadata-trap.md` — Metadata injection (Light + companion)
- `architecture/protected-precomputed.md` — Precomputed variants (Strong/CDN)
- `architecture/protected-steganography.md` — LSB/DCT steganographic embedding

**Source files to verify against:**
- `src/protected/constants.rs` — Constant values and usage
- `src/protected/passthrough.rs` — No-op implementation
- `src/protected/noise.rs` — Noise generation and perturbation
- `src/protected/enhanced.rs` — Enhanced wrapper
- `src/protected/metadata_trap.rs` — Metadata injection (~1283 lines)
- `src/protected/precomputed.rs` — Precomputed cache and variant loader
- `src/protected/steganography.rs` — Steganography (~1915 lines)

**Review focus areas:**
- Verify constant values match between `constants.md` and `constants.rs`
- Verify noise intensity multipliers (10x Standard, 12x Enhanced) in code
- Verify passthrough `is_enabled()` returns false and pipeline skips it correctly
- Verify metadata injection format-specific paths (JPEG EXIF, PNG tEXt, WebP META)
- Verify DMI value mapping (Light->Prohibited, Standard->ProhibitedAiMlTraining, etc.)
- Verify precomputed two-phase registration design
- Verify stego payload format (32 bytes: 24 header + 8 HMAC)
- Verify HMAC uses `subtle::ConstantTimeEq` for timing-attack resistance
- Verify `verify_payload_with_key()` returns `Option<bool>` (not `bool`)
- Check for thread-safety issues in precomputed cache (read lock during apply)
- Check for unbounded cache growth in precomputed protector
- Interrogate: does metadata injection survive encode/decode cycles? What does `apply()` returning `Cow::Borrowed` mean for Light level routing?

**Output format for `plans/protection-strategies.md`:**
```markdown
# Protection Strategies Review Findings

## Document: constants.md
### Verified Claims
### Discrepancies
### Improvement Opportunities
### Potential Bugs/Edge Cases

## Document: protected-passthrough.md
[same structure per document]

## Document: protected-noise.md
[...]

## Cross-Cutting Findings
```

---

### Subagent 3: JPEG Transcoder (`plans/jpeg-transcoder.md`)

**Documents:**
- `architecture/jpeg-transcoder.md` — Transcoder pipeline overview
- `architecture/jpeg-header.md` — JPEG header parser
- `architecture/jpeg-entropy.md` — Huffman entropy codec
- `architecture/jpeg-stego-f5.md` — F5-style DCT steganography

**Source files to verify against:**
- `src/jpeg_transcoder/mod.rs` — Transcoder entry points
- `src/jpeg_transcoder/header.rs` — Header parsing
- `src/jpeg_transcoder/entropy.rs` — Huffman encode/decode
- `src/jpeg_transcoder/stego_f5.rs` — F5 coefficient manipulation

**Review focus areas:**
- Verify `Coefficients` type is `HashMap<u8, Vec<[i64; 64]>>` as documented
- Verify natural (row-major) order storage via `block[ZIGZAG[k]]`
- Verify `get_scan_data_start` uses `checked_add` for overflow protection
- Verify `is_progressive_jpeg` checks SOF2 marker
- Verify Q-table LSB clearing edge case: `&= 0xFE` on value 1 produces 0, clamped back to 1
- Verify seed embedding in quantization tables (12 bytes: 4B magic + 8B seed)
- Verify `F5XorShiftRng` is separate from `XorShiftRng` in util/image.rs
- Verify coefficient clamping (DC max 11 bits, AC max 10 bits)
- Verify byte stuffing handling (0xFF -> 0xFF 0x00)
- Verify restart marker handling (RST0-RST7)
- Verify `debug_assert!` for quantization values > 255 in `assemble_jpeg`
- Check for progressive JPEG edge cases (F5 not supported, seed-only fallback)
- Interrogate: how does the transcoder handle EXIF thumbnails? Trailing garbage? Multiple SOI markers?

**Output format for `plans/jpeg-transcoder.md`:**
```markdown
# JPEG Transcoder Review Findings

## Document: jpeg-transcoder.md
### Verified Claims
### Discrepancies
### Improvement Opportunities
### Potential Bugs/Edge Cases

## Document: jpeg-header.md
[...]

## Document: jpeg-entropy.md
[...]

## Document: jpeg-stego-f5.md
[...]

## Cross-Cutting Findings
```

---

### Subagent 4: Utilities & Integration (`plans/utilities-integration.md`)

**Documents:**
- `architecture/util-image.md` — PRNG, noise gen, perturbation, encoding, hashing
- `architecture/util-iscc.md` — ISCC content identifiers
- `architecture/util-seed.md` — Random seed generation
- `architecture/async-api.md` — Async wrappers (tokio)
- `architecture/cli.md` — CLI tool

**Source files to verify against:**
- `src/util/image.rs` — Core utilities
- `src/util/iscc.rs` — ISCC implementation
- `src/util/seed.rs` — Seed generation
- `src/async_api.rs` — Async wrappers
- `cloakrs-cli/src/main.rs` — CLI implementation

**Review focus areas:**
- Verify `XorShiftRng` XOR offset value matches `XORSHIFT_SEED_OFFSET` constant
- Verify `parallel_threshold()` formula: `cores * 64 * 64`
- Verify `SIN_TABLE` size (256 entries) and lookup behavior
- Verify ISCC perceptual hash algorithm (32x32 grayscale, 2D DCT, 64-bit hash)
- Verify `generate_random_seed()` uses `SystemTime + splitmix64` as documented
- Verify `generate_random_seed().unwrap()` could panic if clock before UNIX epoch
- Verify async batch functions use single `spawn_blocking` (not per-image)
- Verify async single-image functions use one `spawn_blocking` per image
- Verify CLI `--metadata` default (`false`) vs library `inject_metadata` default (`true`) discrepancy
- Verify CLI format auto-detection priority chain
- Verify CLI verification mode extraction order (metadata vs. DCT stego)
- Check for missing cancellation behavior documentation in async API
- Check for missing exit code documentation in CLI
- Interrogate: is the ISCC implementation compatible with the ISCC standard? What are collision rates?

**Output format for `plans/utilities-integration.md`:**
```markdown
# Utilities & Integration Review Findings

## Document: util-image.md
### Verified Claims
### Discrepancies
### Improvement Opportunities
### Potential Bugs/Edge Cases

## Document: util-iscc.md
[...]

## Document: util-seed.md
[...]

## Document: async-api.md
[...]

## Document: cli.md
[...]

## Cross-Cutting Findings
```

---

## Phase 2: Stale Item Pruning

After all 4 subagents complete their reviews, a final pass identifies stale or outdated items in the architecture directory:

### Pruning Criteria

1. **Dead references**: Documents or sections that reference code, types, or functions that no longer exist
2. **Superseded information**: Content that contradicts newer documentation or code (e.g., `TargetModel` concept was removed but may still be referenced)
3. **Incomplete modules**: Documents for modules that were never created or were deleted
4. **Duplicate content**: Information that is redundantly stated across multiple documents without adding value
5. **Outdated cross-references**: Module interaction tables pointing to wrong files or non-existent modules

### Execution

Launch a subagent to:
1. Read all 21 architecture documents
2. Cross-reference every code path, type, function, and module name against the actual `src/` tree
3. Flag any stale items with file, line, and reason
4. Write findings to `plans/stale-items.md`

### Output format for `plans/stale-items.md`:
```markdown
# Stale Architecture Items

## Dead References
- [file]: [line/section] — references [non-existent thing]

## Superseded Information
- [file]: [line/section] — contradicts [newer source]

## Incomplete Modules
- [file] — [reason]

## Duplicate Content
- [file1] vs [file2] — [what is duplicated]

## Outdated Cross-References
- [file]: [table/section] — points to [wrong location]
```

---

## Phase 3: Summary & Consolidation

After Phase 2 completes, produce a final summary:

1. Read all 5 output files in `plans/`
2. Consolidate into `plans/review-summary.md` with:
   - Total discrepancies found per module group
   - Highest-priority bugs or issues
   - Documentation gaps ranked by impact
   - Recommended action items (for a future execution phase)

---

## Execution Notes

- All subagents work within `/Users/davidbowman/projects/cloak/` — no external paths
- Subagents should use `rg` (ripgrep) and `glob` for code search, `read` for file inspection
- Each subagent should be thorough: read every assigned document completely, then verify against source
- The review is read-only — no code changes, only findings written to `plans/`
- Subagents may discover that architecture docs are accurate — that is a valid finding (document "Verified" claims)
- If a subagent finds a potential bug, it should describe the issue and location but NOT propose a fix (this is a review plan, not an execution plan)
