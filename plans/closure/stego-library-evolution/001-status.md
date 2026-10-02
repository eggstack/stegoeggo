# Stego Library Evolution Milestone 001 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/stego-library-evolution/001-boundary-and-metadata-convergence.md`
Source subsystem roadmap: `plans/subsystems/stego-library-evolution-roadmap.md#7-milestones`
Repository baseline reviewed: `d5425d3c73e634fd2276d44507fa94728a18784f` (plan baseline; diff to implementation shows plans-only plus the already-closed maintenance M001 workflow work, no library change)
Implementation commits: `ed1dd6d` (resolved write spec, single executor, explicit JPEG helpers, golden tests, RNG removal, docs reconciliation)
Closing commit: this record plus roadmap/registry reconciliation (see git log for `plans: close stego-library-evolution M001 boundary and metadata convergence`).

## 1. Executive finding

Canonical metadata execution no longer reconstructs any legacy context:
a private `MetadataWriteSpec` carries only the explicit fields format
writers require, legacy and plan entries translate into it exactly once,
and one private executor owns update-policy plus format dispatch. JPEG
helpers and structured-COM rendering consume explicit resolved values.
Emitted bytes are proven identical by six golden SHA-256 hashes captured
before the refactor. Dead root RNG code is removed with grep proof; the
hidden carrier bridge is inventoried at 18 symbols, all retained with
named consumers. No public API, rights semantic, payload byte, or carrier
change. M001 closes and unblocks M002.

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Private resolved metadata-write input with only required fields (§6.1) | `src/protected/metadata_trap/spec.rs`: `MetadataWriteSpec` (metadata pairs, notice, effective DMI, format, policy, limits, structured-COM params); `StructuredComParams` (level byte, seed, intensity); `JpegRender` (limits + structured borrow) (§3) |
| Plan translates directly from `ResolvedProtectionPlan` (§6.2) | `spec_from_plan`: rights-metadata flag, seed-clear on `HiddenMarkerMode::Disabled`, `effective_dmi`, format/policy/limits from plan fields; zero `ProtectionContext` references in the function (§3) |
| Legacy translates through a compatibility translator into the same executor (§6.3) | `spec_from_legacy`: level-default injection flag, `normalize_rights_notice`, legacy legal-claims flag, legacy early-out (`metadata.is_empty`), output/input-format fallback with magic-byte sniff; `inject_bytes` is translate-then-execute (§3) |
| JPEG helpers take explicit values; no canonical-path context construction (§6.4) | `inject_text_chunks_jpeg_with_timestamp` and `inject_all_dmi_markers` take `&JpegRender`; `generate_structured_com_marker_with_timestamp` takes `&StructuredComParams`; repo-wide grep: zero `ProtectionContext::new` in `metadata_trap/` (§3, §4) |
| One implementation owns update-policy + format dispatch (§6.5) | `execute_resolved_write` (private): `FailOnConflict` guard, `PreserveExisting` filtering with suppression, stripping, per-format write; both entries delegate (§3) |
| Remove dead RNG code only with proof (§6.6) | `PixelSelectionRng` + `XORSHIFT_SEED_OFFSET` removed; in-crate grep shows zero consumers, no re-export, `pub(crate)` module; carrier `DctCoefficientRng` sequence still pinned by its own test (§3, §4) |
| Audit hidden bridge; remove only exact duplicates (§6.7) | 18/18 `application_support` symbols inventoried with named consumers in `steganography/{embed,extract,legacy,mod}`; nothing removed (no exact duplicate of a stable generic API found) (§4) |
| No `ProtectionContext` in canonical execution (§13) | Grep proof plus golden plan-path hashes unchanged (§4) |
| One private executor (§13) | `execute_resolved_write` is the only update-policy/format dispatch (§3) |
| Compatible public API and metadata semantics (§13) | Six golden hashes unchanged; public signatures untouched (only private, `#[cfg(test)]`, and one removed test-only shim); full suite green (§4) |
| Counted hidden-support inventory (§13) | 18 symbols, named consumers, recorded in §4 |
| `check.sh` passes (§13) | Exit 0: 2039 passed, 0 failed, 25 ignored across 45 suites (§4) |

## 3. Production implementation evidence

Implementation `ed1dd6d` (16 files, +297/−334):

- `src/protected/metadata_trap/spec.rs` (new): translators plus parameter structs; per-path early-out quirks preserved (`metadata.is_empty()` legacy vs `metadata.is_empty() && dmi.is_none()` + `!rights_metadata` plan).
- `src/protected/metadata_trap.rs`: `inject_bytes` / `inject_bytes_from_plan` reduced to translate-then-execute; new private `execute_resolved_write`; golden regression tests (`canonical_plan_metadata_bytes_are_stable`, `legacy_metadata_bytes_are_stable`).
- `src/protected/metadata_trap/jpeg.rs`: explicit `JpegRender` plumbing; the `#[cfg(test)]` shim keeps its signature and translates internally so all existing direct-helper tests stand unchanged.
- `src/protected/metadata_trap/notice.rs`: structured-COM renderer takes `&StructuredComParams`; removed the test-only ctx shim (single caller updated); dropped the now-unused context import.
- `src/util/image.rs` + `src/protected/constants.rs`: removed `PixelSelectionRng` (struct, impl, two unit tests) and `XORSHIFT_SEED_OFFSET`.
- `stegoeggo-stego/src/jpeg_transcoder/stego_f5.rs`: doc reworded off the retired type; RNG test repurposed as a DCT sequence pin.
- Docs: `architecture/protected-metadata-trap.md` (converged executor), `util-image.md`, `constants.md`, `carrier-lsb.md`, `jpeg-stego-f5.md`, `protected-steganography.md`, `overview.md` (5 spots), plus two `.skills/` factual corrections.

## 4. Verification executed (exact commands + results)

- Golden byte-identity tests (captured pre-refactor, asserted post-refactor):
  plan PNG `6e1592…`, JPEG `3b6567…`, WebP `3a1b3f…`; legacy PNG `9e388c…`, JPEG `e5f290…`, WebP `0cf99f…` — all 6 unchanged.
- `cargo test --workspace --all-features --test request_api` → 14 passed.
- `cargo test --workspace --all-features --test cross_format_semantics` → 34 passed.
- `cargo test --workspace --all-features --test preservation` → 11 passed.
- `cargo test --workspace --all-features --test preservation_idempotence` → 24 passed.
- `cargo test --workspace --all-features --test merge_policy` → 14 passed.
- `cargo test --workspace --all-features --test canonical_rights` → 82 passed.
- `cargo test -p stegoeggo --lib -- protected::metadata_trap::` → 101 passed (incl. both golden tests and all JPEG conflict-policy units).
- `./scripts/check.sh` → exit 0 (fmt, clippy `-D warnings`, no-default-features check, 2039 passed / 0 failed / 25 ignored, docs contract valid).
- `./scripts/verify_metadata_conformance.sh` (non-strict) → 44/44 passed. `--strict` not run: `libvips` is absent in this environment (recorded, not implied); exiftool/xmllint/imagemagick are present.
- Hidden-bridge inventory: 6 functions (`legacy_lsb_required_slots`, `legacy_lsb_extract`, `legacy_lsb_extract_range`, `seed_fallback_embed`, `seed_fallback_extract`, `tile_seed`), `TiledJpegCandidateKey`, `TiledJpegSearch` + 3 methods, `JpegSearchContext` + 6 methods = 18 public items; consumers: `legacy_lsb_*` and `seed_fallback_*` in `steganography/legacy.rs` + `embed.rs` (`embed_raster_with_seed_fallback`, legacy extract paths), `tile_seed` re-exported in `steganography/mod.rs` plus determinism tests, `TiledJpegSearch`/`TiledJpegCandidateKey` in `extract.rs` + `mod.rs` tests, `JpegSearchContext` in `extract.rs` probing/tiling paths.
- Dead-code grep: `PixelSelectionRng` / `XORSHIFT_SEED_OFFSET` → zero hits in `src/`, `stegoeggo-stego/src/`, `tests/`, `benches/`, `examples/`; `cargo check` warning-free.

## 5. Invariant review

- Canonical metadata bytes/semantics: proven identical by goldens for PNG/JPEG/WebP across both entries.
- Update policies exact: single executor preserves `FailOnConflict` error text, `PreserveExisting` filtering/suppression, and `ReplaceStegoOwned` stripping with per-path DMI sources (`notice.dmi()` legacy, `plan.effective_dmi()` plan).
- Byte APIs remain the only metadata-claiming path: `apply()` still returns `Cow::Borrowed` unchanged; no pixel-path change.
- Legacy callers functional through 0.x: `inject_bytes` keeps its `#[doc(hidden)]` signature and quirks; public legacy adapters untouched.
- V1/V2/V3 payload bytes and carrier selection: untouched (no steganography-module change except doc-adjacent test rename in `stego_f5.rs`).
- No public visibility increase: `spec` module and executor are private; only change to `pub` surface is removing a `#[cfg(test)]` shim.
- `#![forbid(unsafe_code)]` retained in both crates; `cargo fmt` 4-space/100-col clean.

## 6. Failure and recovery review

Metadata writes remain all-or-error `Vec<u8>` transformations: conflict errors return before allocation, stripping failures propagate, and no partial output escapes on render failure. Bounded parsing is preserved (`JpegRender.limits` threads the same `ResourceLimits` through field-count and XMP-size gates). The refactor stopped at no unexpected byte difference — goldens matched on first post-refactor run, so no fixture reclassification was needed.

## 7. Migration and compatibility review

No public migration: no new fields on legacy context, no serialization change, no codec internals moved into the carrier API. The legacy `ProtectionContext` path keeps byte-identical output (golden legacy hashes). Hidden support stays hidden; nothing application-specific was promoted.

## 8. Security review

No secret handling changed (seed remains `u64`, never logged). Structured-COM level byte defaults preserved (legacy `None` level → 2; plan path → 2). Bounded JPEG segment parsing and resource-limit gates are structurally unchanged, now fed from the spec instead of a reconstructed context.

## 9. Documentation and operations

`architecture/protected-metadata-trap.md` now describes the spec/executor convergence; six further deep-dives plus `overview.md` no longer describe context reconstruction or the retired RNG; `docs/rust-api.md` needed no change (no reconstruction wording present); two `.skills/` factual corrections keep agent instructions truthful. No operator action required.

## 10. Unresolved findings (critical/high/medium/low)

None. One environmental note (not a finding): strict external conformance needs `libvips`, absent here; non-strict conformance is 44/44 and byte-identity goldens cover the refactor's risk directly.

## 11. Roadmap disposition

Stego-library-evolution M001 closed. M002 (generic carrier API normalization) is unblocked: its hard M001 dependency is satisfied with no interface change outstanding, so it promotes to ready. M003–M006 remain blocked behind M002/M003/ADR-0007 as recorded.

## 12. Registry updates

`registry.md`: M001 row removed from dependency-ready plans; stego M002 promoted blocked→ready with its plan linked; subsystem current milestone advanced to M002; M001 recorded under recently closed work. Roadmap status table marks M001 closed with this closure record.
