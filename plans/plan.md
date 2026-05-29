# Architecture Documentation Fix Plan

## Status: Complete

All documentation discrepancies between `architecture/*.md` docs and the `cloakrs` codebase have been fixed. The architecture docs now accurately reflect the source code.

**Completed**: 2026-05-29 (all waves + follow-up fixes)

---

## Known Bugs & Edge Cases (Open)

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
