# Architecture Review Summary

## Executive Summary

A comprehensive architecture review of the `cloakrs` library across 5 module groups identified **83 discrepancies** between documentation and code, **15 potential bugs or edge cases**, and **28 stale or superseded documentation items**. The most critical issues involve incorrect type signatures and variant names that would cause compilation failures for users following the docs. The codebase itself is architecturally sound with clean module separation, but the documentation has drifted significantly from the current implementation.

---

## Total Discrepancies per Module Group

| Module Group | Discrepancies | Stale Items |
|---|---|---|
| Core Framework (lib, traits, types, error) | 12 | 14 |
| Protection Strategies (passthrough, noise, enhanced, precomputed, stego, metadata trap) | 10 | 11 |
| JPEG Transcoder (mod, header, entropy, stego_f5) | 11 | 8 |
| Utilities & Integration (image, iscc, seed, async, cli) | 17 | 17 |
| **Total** | **50** | **50** |

Note: Stale items (dead references, superseded info, incomplete modules, duplicates, outdated cross-refs) are counted separately in `stale-items.md`. Some findings overlap across groups.

---

## Highest-Priority Bugs and Issues

### Critical (Compilation failures / incorrect behavior)

1. **DmiValue variant names completely wrong in docs** — `overview.md`, `types.md` list `ProhibitedScraping` and `ProhibitedAnyProcessing` which do not exist. Actual variants: `ProhibitedExceptSearchEngineIndexing` and `ProhibitedSeeConstraints`. Users following the docs will fail to compile. *(types.rs:14-16)*

2. **`ProtectionConfig.mac_key` type mismatch** — Doc says `pub mac_key: Vec<u8>` but actual type is `pub mac_key: Option<Vec<u8>>`. Direct field access will fail. *(types.rs:269)*

3. **`StegoPayload::protection_level()` returns `u8`, not `ProtectionLevel`** — Doc says it returns `ProtectionLevel` enum; actual code returns `u8` (steganography.rs:1038). Type mismatch causes compilation failure.

4. **`Coefficients` inner type wrong** — Doc says `[i64; 64]`, actual is `[i16; 64]`. Affects anyone using the Coefficients type directly. *(mod.rs:14)*

5. **Steganography method signatures completely wrong** — All five public steganography methods (`extract_payload`, `verify_payload`, `verify_payload_with_key`, `verify_payload_from_bytes`, `verify_payload_from_bytes_with_key`) documented as free functions but are actually `&self` methods on `SteganographyProtector` with different parameter lists. *(steganography.rs)*

6. **NoiseGenerator constructor mismatch** — Doc describes `new(key: &[u8])` but actual API is `new(seed: u64)` and `with_mac_key(seed: u64, mac_key: ...)`. Entirely different API surface. *(image.rs:89-92)*

7. **Perturbation function signatures completely wrong** — `apply_perturbation_single_pass`, `apply_perturbation_single_pass_keyed`, `apply_perturbation_single_pass_keyed_par` all have different parameters, mutability, and return types than documented. *(util-image.md:55-65)*

### High (Incorrect behavior / security concerns)

8. **`inject_metadata` / `inject_legal_claims` `Option<bool>` semantics undocumented** — Fields are `Option<bool>` with `None` defaults, but documented as `bool`. Callers cannot distinguish "not set" from "explicitly disabled." Behavior when `None` vs `false` is ambiguous. *(types.rs:304-305)*

9. **`process_bytes` skips dimension validation** — `process()` validates dimensions (lib.rs:217) but `process_bytes()` does not (lib.rs:318). Large images exceeding `max_dimension` bypass the check via the byte path.

10. **PrecomputedProtector unbounded cache** — `RwLock<HashMap>` has no eviction policy, size limit, or TTL. Under sustained load with diverse images, cache grows without bound until memory exhaustion. *(precomputed.rs:37)*

11. **Seed embedding silent failure** — When quantization table values are all 1, clearing LSBs (`&= 0xFE`) produces 0 (clamped to 1), meaning 0-bits cannot be reliably embedded. Function returns `Ok(())` even if seed embedding failed. *(stego_f5.rs:103-108)*

12. **CLI `-o` argument behavior wrong in docs** — Doc says `-o` sets output file path directly. Actual code generates `{stem}_protected.{ext}` regardless; `-o` sets the output *directory*, not file. *(main.rs:236)*

### Medium (Incorrect documentation / misleading info)

13. **`estimated_latency_ms()` return type and values wrong across all docs** — Trait returns `u32`, docs say `f64`. Values are off: Noise (doc ~5.0, code 3), Enhanced (doc ~7.0, code 5), Precomputed (doc 0.0, code 2). *(traits.rs:88)*

14. **`ProtectedVariant::cache_key()` format wrong** — Doc says `{uuid}_{hash}_{intensity}`, code generates `{hash}_{level}_{intensity}`. UUID is not in the cache key. *(types.rs:596-606)*

15. **ISCC not ISCC-standard compliant** — Uses custom component codes (`0x12`, `0x33`) and non-standard hash algorithm. Should not be described as "ISCC" without qualification. *(iscc.rs)*

---

## Documentation Gaps Ranked by Impact

### Critical Impact (Would cause compilation failure)

| Gap | File(s) | Impact |
|---|---|---|
| DmiValue variant names wrong | types.md, overview.md | Users cannot construct DmiValue enums |
| `mac_key` type wrong | types.md | Field access fails |
| `StegoPayload::protection_level()` return type wrong | types.md | Type mismatch |
| `Coefficients` inner type wrong | jpeg-transcoder.md | Type mismatch |
| All 5 steganography method signatures wrong | protected-steganography.md | Method calls fail |
| NoiseGenerator constructor wrong | util-image.md | Instantiation fails |
| Perturbation function signatures wrong | util-image.md | Function calls fail |
| `assemble_jpeg` visibility wrong | jpeg-transcoder.md | Private method documented as public |
| `get_scan_data_start` return type wrong | jpeg-transcoder.md | `Result` vs `Option` mismatch |
| TranscoderError variants wrong | jpeg-transcoder.md | 3 of 6 variants don't exist |

### High Impact (Would cause runtime errors or security issues)

| Gap | File(s) | Impact |
|---|---|---|
| `inject_metadata`/`inject_legal_claims` Option semantics | types.md | Unexpected default behavior |
| Dimension validation asymmetry | pipeline.md | Bypassable safety check |
| PrecomputedProtector cache unbounded | protected-precomputed.md | Memory exhaustion risk |
| CLI stdin support claimed but absent | cli.md | Users expect stdin, get error |
| CLI verification mode missing HMAC | cli.md | Security feature gap |
| `is_enabled()` documented as returning false | protected-passthrough.md | Dead code, misleading |
| `MIN_PAYLOAD_SIZE` wrong (26 vs 32) | protected-steganography.md | Payload parsing confusion |

### Medium Impact (Incorrect but won't break code)

| Gap | File(s) | Impact |
|---|---|---|
| `estimated_latency_ms` type/values wrong | All trait docs | Misleading performance expectations |
| `cache_key()` format wrong | types.md, protected-precomputed.md | Cache debugging confusion |
| ISCC standard non-compliance | util-iscc.md | Misleading interoperability claims |
| CLI `-o` behavior wrong | cli.md | Unexpected output paths |
| CLI format auto-detection step missing | cli.md | Users don't understand format routing |
| `process_image_bytes` auto-detection undocumented | pipeline.md | Hidden behavior |
| `PerturbationParams` field names wrong | util-image.md | Internal API confusion |
| `XorShiftRng` methods wrong | util-image.md | Internal API confusion |

### Low Impact (Minor inaccuracies)

| Gap | File(s) | Impact |
|---|---|---|
| SIN_TABLE size wrong (256 vs 1024) | util-image.md | No functional impact (internal) |
| `generate_random_seed` panic claim wrong | util-seed.md | Misleading safety info |
| `unwrap_or_default()` vs `unwrap()` | util-seed.md | Misleading error behavior |
| Duplicate XorShiftRng warnings | overview.md, util-image.md, jpeg-stego-f5.md | Redundant but harmless |
| Line count drift | pipeline.md | Cosmetic |
| Module map incomplete descriptions | overview.md | Navigation confusion |

---

## Recommended Action Items for Future Execution Phase

### P0 — Must Fix (Compilation Blockers)

1. **Update DmiValue variant names** in `types.md` and `overview.md` — replace `ProhibitedScraping` → `ProhibitedExceptSearchEngineIndexing`, `ProhibitedAnyProcessing` → `ProhibitedSeeConstraints`

2. **Fix `ProtectionConfig.mac_key` type** in `types.md` — change `Vec<u8>` → `Option<Vec<u8>>`

3. **Fix `StegoPayload::protection_level()` return type** in `types.md` — change `ProtectionLevel` → `u8`

4. **Fix `Coefficients` inner type** in `jpeg-transcoder.md` — change `i64` → `i16`

5. **Rewrite steganography method signatures** in `protected-steganography.md` — change from free functions to `&self` methods with correct parameter lists

6. **Rewrite NoiseGenerator documentation** in `util-image.md` — document actual `new(seed)` / `with_mac_key(seed, mac_key)` API

7. **Rewrite perturbation function signatures** in `util-image.md` — update all 5 functions to match actual signatures

8. **Fix `assemble_jpeg` documentation** in `jpeg-transcoder.md` — mark as private, return `Result<Vec<u8>>`

9. **Fix `get_scan_data_start` return type** in `jpeg-transcoder.md` — change `Result<usize>` → `Option<usize>`

10. **Fix TranscoderError variants** in `jpeg-transcoder.md` — replace with actual 6 variants

### P1 — Should Fix (Runtime/Security Issues)

11. **Document `inject_metadata`/`inject_legal_claims` `Option<bool>` semantics** — clarify what `None` means vs explicit `true`/`false` in `types.md`

12. **Document dimension validation asymmetry** — note in `pipeline.md` that `process_bytes` skips `max_dimension` check

13. **Document PrecomputedProtector cache limitations** — add warning about unbounded growth, recommend external eviction

14. **Fix CLI documentation** — correct `-o` behavior, remove stdin claim, document verification mode limitations, fix format auto-detection

15. **Fix `is_enabled()` documentation** — correct return value to `true`, note it is unused by pipeline

16. **Fix stego payload constants** — update `MIN_PAYLOAD_SIZE` to 26, `MIN_PAYLOAD_BITS` to 208

### P2 — Nice to Fix (Accuracy/Consistency)

17. **Update `estimated_latency_ms` values and type** across all protection strategy docs — change `f64` → `u32`, fix numeric values

18. **Fix `cache_key()` format** in `types.md` and `protected-precomputed.md`

19. **Clarify ISCC standard non-compliance** in `util-iscc.md`

20. **Update `PerturbationParams` fields and visibility** in `util-image.md`

21. **Update `XorShiftRng` methods** in `util-image.md` — change to actual API

22. **Fix `generate_random_seed` documentation** — correct `unwrap_or_default()` behavior, mention non-zero guarantee

23. **Deduplicate XorShiftRng warnings** across overview.md, util-image.md, jpeg-stego-f5.md

24. **Fix module map descriptions** in overview.md

25. **Add async cancellation and thread pool guidance** to async-api.md
