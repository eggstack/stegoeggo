# Verification and Conformance Milestone 006 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/verification-conformance/006-unstructured-candidate-false-positive-corrective.md`
Source subsystem roadmap: `plans/subsystems/verification-conformance-roadmap.md#m6--unstructured-candidate-false-positive-corrective`
Repository baseline reviewed: `948f61a973faeef60c851bef07b64fac149ffeaa`
Implementation commits: `8d499c8` — absence-vs-corruption corrective implementation; `990daa4` — test formatting follow-up

## 1. Executive finding

The canonical absence-versus-corruption invariant is restored. A valid image with no credible StegoEggo hidden-marker evidence reports hidden-marker `VerificationStatus::NotFound`; provenance-bearing corrupted, malformed, unsupported-version, authentication-failed, key-missing, and resource-limited candidates still report `Invalid` with unchanged diagnostics. The regression caught independently by the Python and Node compatibility suites is fixed in canonical Rust, and Rust-level regression guards now cover the exact false-positive class so foreign-binding CI is no longer the only detector.

## 2. Requirement-to-evidence matrix

| Plan requirement | Evidence |
|---|---|
| Unprotected canonical images report `NotFound` | `canonical_independent_fixtures_report_absence` (PNG/JPEG/WebP) and `unstructured_fallback_bytes_report_absence_not_corruption` in `tests/verification_convergence.rs`; before fix the PNG fixture returned `Invalid` with diagnostic "Payload found but integrity check failed", after fix all three return `NotFound` |
| Metadata-only images keep rights with marker `NotFound` | `metadata_only_black_image_has_rights_without_marker`; existing matrix case `metadata_only` in `verification_matrix_cross_api_consistency` |
| Structured corruption still reports `Invalid` | `wrong_hmac_key_stays_invalid_on_black_image` (correct key `Verified`, wrong/missing key `Invalid`); existing HMAC and V3 integrity unit tests in `src/protected/steganography/verify.rs` |
| Wrong/missing HMAC-key behavior stays distinct | Same test as above; canonical `AuthKeyMissing`/`AuthFailed` outcome arms and report diagnostics unchanged |
| V3 malformed/unsupported stays diagnostic | `MalformedV3`/`UnsupportedVersion` outcome arms and report diagnostics unchanged; `classify_v3_prefix`/`validate_v3_header` unit tests unchanged and passing |
| Seed 0 remains valid | `seed_zero_marker_verifies_on_black_image` (`Verified` on the exact image class that false-positived) |
| Verification/resource budgets stay enforced | Rewritten `payload_limit_rejects_large_payload` (`tests/plan026_gate1_2_3_tests.rs`): real protected marker verified with `max_payload_bytes(10)` reports `Invalid`, not `NotFound`; `ResourceLimitExceeded` now returns immediately from redundancy/origin search instead of being shadowed |
| Rust-level regression guards for the false-positive class | Five new tests in `tests/verification_convergence.rs` (11/11 pass); no reliance on foreign bindings |
| `./scripts/check.sh` green | Local exit 0 (45 `test result: ok` suites); remote CI run `36916901430` success on `990daa4` |
| Downstream false-positive assertions pass | Python binding CI run `36916208734`: `test_verify_unprotected_image_has_no_rights` passes; Node binding CI run `36916208770`: `verify reports an unprotected image` passes (63 pass / 1 fail, sole failure is the language-owned rights-source label) |

## 3. Production implementation evidence

- `src/protected/steganography/mod.rs`: new `CandidateProvenance` (`Explicit` for metadata/Q-table seeds, `Unstructured` for LSB fallback and test-seed sweeps) threaded through every `verify_payload_from_bytes_outcome` probe site, including an explicit identity pass for the JPEG DCT paths.
- `src/protected/steganography/verify.rs`: removed the dead `_suppress_unstructured_candidates` boolean; added `payload_is_structurally_plausible` (ECC-decode-then-parse oracle, V3 magic included) and `filter_candidate_outcome`, which downgrades only non-credible unstructured `Invalid` to `NotFound`. No seed-value heuristic; `Valid`, `AuthenticationKeyMissing`, `AuthenticationFailed`, `MalformedV3`, `UnsupportedVersion`, and `ResourceLimitExceeded` are never downgraded.
- `src/protected/steganography/extract.rs`: `ResourceLimitExceeded` returns immediately from `verify_extract_with_redundancy` and `verify_extract_lsb_tiled` instead of being shadowed by earlier garbage-candidate `Invalid` outcomes.
- `src/verification/canonical.rs`: call-site update for the removed boolean parameter; projection logic unchanged.
- `architecture/verification.md`: absence-vs-corruption invariant documented where it was previously implicit.

## 4. Verification executed (exact commands + results)

- `./scripts/check.sh` — local exit 0, 45 suites `ok`, zero failures; remote CI run `36916901430` (SHA `990daa4`) success.
- `cargo test --workspace --exclude stegoeggo-fuzz --all-features --test verification_convergence` — 11 passed, 0 failed.
- `cargo test --workspace --exclude stegoeggo-fuzz --all-features --test plan026_gate1_2_3_tests` — 57 passed, 0 failed.
- `cargo test --workspace --exclude stegoeggo-fuzz --all-features --test verification_report_tests` — 19 passed, 0 failed (via `check.sh`).
- `cargo test --workspace --exclude stegoeggo-fuzz --all-features --test detached_manifest_tests` — 53 passed, 0 failed.
- Node binding Rust units: `cargo test --manifest-path bindings/node/Cargo.toml --locked` — 54 passed.
- Python binding (local wheel build + install): 70 passed, 1 failed (`test_metadata_limit_exceeded_structured_attributes` kind wording, owned by language-bindings M006); remote run `36916208734`: 69 passed, 1 skipped, 1 failed (same single kind-wording failure).
- Node binding JS (local): 63 pass, 1 fail (`verify reports a metadata-only image` on `rights.source` label, owned by language-bindings M006); remote run `36916208770`: identical 63 pass / 1 fail.
- `pnpm typecheck` and generated-declaration drift check (`git diff --exit-code -- index.js index.d.ts`) pass locally.

## 5. Invariant review

- `verify_image_bytes_report` remains canonical; all other verification types project from the same facts (convergence suite green).
- No credible marker evidence on a valid carrier yields hidden-marker `NotFound`, not `Invalid` (proven by the new fixture/black-image tests).
- `Invalid` remains observable for structured corruption (wrong-HMAC test) and resource-limit exhaustion (rewritten payload-limit test).
- Missing-key vs wrong-key stays distinguishable; malformed/unsupported V3 stays diagnostic; legacy V1/V2 support untouched.
- Seed 0 verifies (dedicated test); no numeric-seed heuristic exists anywhere in the new code.
- Budgets remain bounded; no extra carrier scan pass was added (filter applies to already-computed outcomes).
- `#![forbid(unsafe_code)]` unchanged; bindings consume Rust results verbatim (no wrapper reinterpretation in this milestone).

## 6. Failure and recovery review

- Ambiguous candidates without StegoEggo structure fail toward `NotFound` by construction of the filter.
- Explicit-prefix resource exhaustion is preserved via immediate `ResourceLimitExceeded` return.
- No unbounded retry was added; failed candidates do not contaminate later probes (each probe outcome is independent; filter is a pure post-step).
- Candidate ordering no longer changes the resource-limit classification (immediate return is order-independent).
- A later valid candidate can still establish `Verified`: the filter only converts non-credible `Invalid` to `NotFound`, and the search continues past `NotFound`.

## 7. Migration and compatibility review

Regression correction, not a new public semantic. No serialized report schema change; `CandidateProvenance` and the filter are `pub(crate)`. Two existing tests relied on the buggy unstructured-`Invalid` behavior and were reconciled against the accepted model rather than silently edited:

- `tests/detached_manifest_tests.rs` (`test_embedded_reference_raw_payload_none`, `test_embedded_reference_wrong_digest_reports_stripped_without_payload`): solid-black unprotected image with a manifest-declared embedded reference now yields `Stripped` (never present) instead of `Malformed` (previously reached only via the false-positive `Invalid`).
- `tests/plan026_gate1_2_3_tests.rs` (`payload_limit_rejects_large_payload`): the old version used a blank image with no payload, so its `Invalid` came from the false positive rather than any limit. It now protects a real marker and verifies with `max_payload_bytes(10)`, genuinely exercising limit enforcement (`Invalid` via `ResourceLimitExceeded`).

## 8. Security review

Constant-time HMAC comparison untouched; bounded verification preserved (seed/origin caps unchanged; immediate resource-limit return shortens wasted work). No secret material in new code paths (filter carries no key bytes; new tests use non-secret matrix keys).

## 9. Documentation and operations

- `architecture/verification.md` now states the absence-vs-corruption rule and its code locations.
- No CLI, release, or CI contract changes. Required CI (`scripts/check.sh`) stays the sole required gate.

## 10. Unresolved findings (critical/high/medium/low)

- None for this milestone. Two downstream binding-test drifts observed during verification are owned by language-bindings M006, not this corrective: Python metadata-limit `kind` wording (`metadata field` vs frozen `tEXt field`) and Node metadata-only `rights.source` label (`xmp` vs frozen `legacy`). Both are binding-expectation reconciliations; canonical Rust behavior is asserted correct.

## 11. Roadmap disposition

Verification-conformance M6 is closed. The subsystem returns to closed: all milestones M1–M6 are historical/closed evidence. Language-bindings M006's interface dependency (absence-vs-corruption contract) is satisfied and its closure gate on verification M006 is released; language-bindings M006 implementation may proceed to closure.

## 12. Registry updates

- `plans/registry.md`: verification-conformance M006 `active` → `closed`; subsystem `verification-conformance` `active` → `closed`.
- `plans/subsystems/verification-conformance-roadmap.md`: status `active` → `closed`; M6 `active` → `closed` with closure link.
- `plans/registry.md`: language-bindings M006 note updated to record verification M006 closure (M007 remains blocked on language M006 closure).
