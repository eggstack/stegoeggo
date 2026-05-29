# Architecture Review Plan

## Overview

This plan orchestrates a systematic review of all architecture documents in the `architecture/` directory. Each module is assigned to a subagent that will:
1. Review the architecture document(s)
2. Verify claims against the actual source code
3. Identify bugs, inconsistencies, and improvement opportunities
4. Write findings to `plans/<module>-review.md`

After all reviews complete, stale items in `architecture/` will be identified and pruned.

---

## Module Assignments

### Module 1: Core Pipeline & Overview
**Documents**: `architecture/overview.md`, `architecture/pipeline.md`
**Source**: `src/lib.rs`
**Review Output**: `plans/core-pipeline-review.md`

### Module 2: Core Types & Traits
**Documents**: `architecture/types.md`, `architecture/traits.md`
**Source**: `src/types.rs`, `src/traits.rs`
**Review Output**: `plans/types-traits-review.md`

### Module 3: Error Handling
**Documents**: `architecture/error.md`
**Source**: `src/error.rs`
**Review Output**: `plans/error-review.md`

### Module 4: Protected Modules
**Documents**: `architecture/protected-*.md` (noise, enhanced, precomputed, passthrough, metadata_trap, steganography)
**Source**: `src/protected/` (mod.rs, noise.rs, enhanced.rs, precomputed.rs, passthrough.rs, metadata_trap.rs, steganography.rs)
**Review Output**: `plans/protected-review.md`

### Module 5: JPEG Transcoder
**Documents**: `architecture/jpeg-*.md` (header, entropy, stego-f5, transcoder)
**Source**: `src/jpeg_transcoder/` (mod.rs, header.rs, entropy.rs, stego_f5.rs)
**Review Output**: `plans/jpeg-transcoder-review.md`

### Module 6: Utilities & Constants
**Documents**: `architecture/util-*.md` (image, seed, iscc), `architecture/constants.md`
**Source**: `src/util/` (mod.rs, image.rs, seed.rs, iscc.rs), `src/protected/constants.rs`
**Review Output**: `plans/utilities-review.md`

### Module 7: Async API & CLI
**Documents**: `architecture/async-api.md`, `architecture/cli.md`
**Source**: `src/async_api.rs`, `cloakrs-cli/src/main.rs`
**Review Output**: `plans/async-cli-review.md`

---

## Stale Item Detection

After all module reviews complete, scan `architecture/` for:
1. Documents without corresponding source files (deleted/renamed modules)
2. Source files without architecture documents (new modules lacking docs)
3. Documents that reference removed types/functions (outdated claims)
4. Discrepancies between documented behavior and actual implementation

**Output**: `plans/stale-items.md`

---

## Execution Order

1. Launch all 7 module review subagents in parallel
2. Wait for all reviews to complete
3. Compile stale item report
4. Commit all review outputs to main

---

## Review Methodology

Each subagent must:
1. Read the architecture document(s) for their module
2. Read the corresponding source files
3. For each claim/concept in the doc:
   - Find the corresponding code
   - Verify the claim is accurate
   - Note any discrepancies
4. For each source file:
   - Check for bugs (logic errors, edge cases, panics)
   - Check for improvement opportunities (performance, safety, clarity)
   - Verify error handling is comprehensive
5. Write findings to the designated `plans/<module>-review.md`

**Findings format**:
- **Verified Claims**: What works as documented
- **Discrepancies**: Where docs don't match code
- **Bugs Found**: Specific issues with line references
- **Improvement Opportunities**: Refactoring, optimization, safety suggestions
- **Stale References**: Outdated function/type names in docs
