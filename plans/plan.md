# Cloakrs Bug Fix & Improvement Plan

## Status: Complete

All tasks from this plan have been implemented and verified (2026-05-29).

| Task | Commit | Description |
|------|--------|-------------|
| Dimension validation in `process_bytes` | `321f825` | Validates max_dimension for both JPEG (header parse) and non-JPEG byte paths |
| LRU eviction for PrecomputedProtector | `14121aa` | `lru` 0.12 crate, default capacity 100, configurable via `with_capacity()` |
| Seed embedding unit quant error | `ef5c249` | Precondition check fails if any quantization value < 2 |
| `Option<bool>` documentation | `c296213` | Three-state semantics documented on fields, builders, and getters |
| CLI batch filename collisions | `237fe23` | `HashMap<PathBuf, usize>` collision tracking, `_protected_1` suffixes |

Full test suite: 264 tests pass (168 unit + 20 basic + 63 integration + 9 async). Clippy clean. Format clean.
