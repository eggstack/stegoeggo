# Stego Library Evolution Milestone 008 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/stego-library-evolution/008-m003-default-limit-compatibility-corrective.md`
Source subsystem roadmap: `plans/subsystems/stego-library-evolution-roadmap.md#7-milestones`
Repository baseline reviewed: `0a8b356a056bdab2f9d1996be78578cb3f6bc724`
Implementation commits: `4f170dca38400d9e7b83edf554b8320409bf3169` — restore legacy JPEG input/dimension compatibility while retaining explicit bounded behavior
Closing commit: recorded in git history for the planning reconciliation commit.

## 1. Executive finding

M008 is complete. Legacy JPEG operations use a private compatibility profile with the v0.4.2 structural domain and historical parser-error mapping. Explicit `*_with_limits`, `inspect_with_limits`, `probe_support_with_limits`, and `PreparedJpeg::new_with_limits` retain the `CarrierLimits` contract and `ResourceLimitExceeded` classification. The defaults were not widened. Root `ResourceLimits` remain enforced before canonical JPEG carrier work. The latest public release was still v0.4.2 when this correction was completed.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Classify the current public JPEG / prepared operation surface by v0.4.2 history | History matrix below, checked against `git show v0.4.2:stegoeggo-stego/src/jpeg.rs`, `git show v0.4.2:stegoeggo-stego/src/prepared.rs`, and the v0.4.2 crate manifest. `inspect`, `is_progressive_jpeg`, `PreparedJpeg::new`, and all methods on the then-public `PreparedJpeg` existed; most current public JPEG one-shot carrier methods and all `_with_limits` variants are later API additions. |
| Restore historical structural input / dimension behavior for compatibility operations | `CarrierLimits::legacy_compatibility()` pins addressable input (`usize::MAX`), JPEG SOF width/height (`u16::MAX`), checked structural pixel product, 256 segments, 65,535 bytes per segment, default frame maximum, and `MAX_TILED_ORIGINS`; private-profile unit test pins those values. |
| Keep explicit bounded APIs and parser-exhaustion classification | Legacy and bounded parser fixture asserts `capacity()` returns `MalformedInput` while `capacity_with_limits(..., CarrierLimits::default())` returns `ResourceLimitExceeded`. `inspect()` with caller segment limit returns `MalformedInput`; `inspect_with_limits()` returns `ResourceLimitExceeded`. Existing bounded adversarial tests remain green. |
| Cross the 16,384 dimension threshold at low pixel count | `legacy_jpeg_surface_preserves_structural_dimension_domain` uses a supported 20,000×1 JPEG (44,852 bytes): legacy inspect, support probe, capacity, and `PreparedJpeg::new` succeed; corresponding default-bounded inspect/probe/capacity/prepared construction return `ResourceLimitExceeded`. |
| Preserve framed / tiled structural limits and outputs | Compatibility profile inherits the existing frame limit and `MAX_TILED_ORIGINS`; framed legacy wrappers retain the frame limit, and tiled wrappers retain origin validation. Known-answer vectors and existing M003/M005 suites pass. |
| Keep application resource policy bounded | `src/pipeline.rs::process_plan_bytes` checks root input size before parsing, invokes `inspect` with root JPEG segment limits, and checks root dimensions before carrier work. `cargo test --workspace --all-features --test request_api max_dimensions_enforced_on_jpeg_to_jpeg_fast_path` passes. Existing `tests/robustness.rs` bounded-variant coverage also passes. |
| Reconcile docs and planning dependencies | Carrier README/rustdoc, `docs/carrier-crate.md`, `STABILITY.md`, JPEG/resource architecture, changelog, roadmap, registry, and this closure record describe the two contracts. M006 no longer lists M008 as a blocker; M007 remains proposed pending explicit maintainer authorization. |

### API-history / behavior matrix

| Current public operation | Public in v0.4.2? | v0.4.2 behavior / limits | Post-corrective disposition |
|---|---:|---|---|
| `jpeg::inspect(bytes, max_segments, max_segment_bytes)` | Yes | Caller-provided segment count/size limits only; no input or dimension policy cap; parser failures were `MalformedInput` | Compatibility profile with caller segment limits; dimensions up to JPEG SOF domain; parser failures remain `MalformedInput` |
| `jpeg::is_progressive_jpeg` | Yes | Header parse with parser defaults; no M003 input/dimension policy | Unchanged; it does not use `CarrierLimits` |
| `PreparedJpeg::new` | Yes | 256 JPEG segments / 65,535 bytes per segment; no 100 MiB or 16,384 dimension cap; parse errors map to `MalformedInput` | Uses private compatibility profile; frame/origin operation limits remain unchanged |
| `PreparedJpeg::{support,capacity,extract,extract_framed,extract_tiled,extract_tiled_framed,embed_strict,embed_framed_strict,seed_hint}` | Yes | Methods operated on state produced by `new`; historical JPEG parser domain applied at construction; existing frame/tiled bounds applied to those operations | Reuse prepared compatibility decode; default frame and origin bounds remain in force |
| `jpeg::{probe_support,capacity,embed,embed_best_effort,embed_strict,embed_framed,embed_framed_best_effort,embed_framed_strict,extract,extract_framed,embed_tiled,embed_tiled_framed,extract_tiled,extract_tiled_framed}` | No; these were not public in v0.4.2 source | No published public API contract at v0.4.2; internal operations used the structural JPEG parser domain | Routed consistently through compatibility semantics; existing carrier bytes and embed/extract algorithms are unchanged |
| `*_with_limits`, `inspect_with_limits`, `probe_support_with_limits`, `PreparedJpeg::new_with_limits` | No; introduced with M003 | No pre-M003 caller contract | Remain bounded by default or caller-provided `CarrierLimits`; budget exhaustion remains `ResourceLimitExceeded` |
| WebP carrier operations | No; introduced by M005 | No pre-M003 compatibility surface | Remain explicitly bounded and unchanged |

## 3. Production implementation evidence

- `stegoeggo-stego/src/limits.rs`: private `legacy_compatibility()` profile, derived pixel maximum uses checked multiplication, and a unit test pins the structural and preserved parser/frame/tiled limits. `CarrierLimits::default()` is unchanged.
- `stegoeggo-stego/src/jpeg.rs`: shared decode/profile plumbing distinguishes legacy parser-error mapping from explicit bounded mapping. Compatibility operations route through the private profile; explicit bounded variants retain their caller limits. No coefficient decoder or support classifier was forked.
- `stegoeggo-stego/src/prepared.rs`: `PreparedJpeg::new` uses compatibility decode; `new_with_limits` retains bounded construction and stores caller limits.
- `stegoeggo-stego/tests/direct_consumer.rs`: wide low-pixel dimension test and parser-error disposition regression.
- Documentation now describes compatibility and bounded contracts separately. No 100+ MiB routine test fixture was added; the private profile test pins the addressable input ceiling as required.

## 4. Verification executed (exact commands + results)

- `cargo test -p stegoeggo-stego --all-features` → 281 passed across carrier unit, direct-consumer, tiled allocation, WebP, and doc-test suites; 0 failed.
- `cargo test --workspace --all-features --test public_stego_api` → 56 passed, 0 failed.
- `cargo test --workspace --all-features --test known_answer_vectors` → 10 passed, 0 failed.
- `cargo test --workspace --all-features --test robustness` → 71 passed, 0 failed.
- `cargo test --workspace --all-features --test request_api max_dimensions_enforced_on_jpeg_to_jpeg_fast_path` → 1 passed; root configured dimension policy still rejects before processing.
- `cargo semver-checks check-release -p stegoeggo-stego` → 196 checks pass, 58 skip; summary: no semver update required. This checks API shape only and does not establish behavioral compatibility.
- `./scripts/check.sh` → exit 0: formatting, clippy with `-D warnings`, no-default-features check, workspace tests, doc tests, and docs contract check all green. The workspace run includes the root resource-limit regression and the M005 bounded WebP suite.
- v0.4.2 behavior comparison worktree/harness: `git worktree add --detach /tmp/stegoeggo-v042 v0.4.2`; temporary `m008_v042_comparison` integration harness; `cargo test -p stegoeggo-stego --all-features --test m008_v042_comparison -- --nocapture` → 1 passed. v0.4.2 `inspect` and `PreparedJpeg::new` accepted the same 20,000×1 / 44,852-byte supported JPEG and `PreparedJpeg::new` classified a 256-segment-overflow fixture as `MalformedInput`. Temporary worktree/harness were removed after comparison.
- Latest release check: `gh api repos/eggstack/stegoeggo/releases/latest --jq '{tag_name: .tag_name, published_at: .published_at}'` → `v0.4.2`, published `2026-09-22T14:13:47Z`.

## 5. Invariant review

- Legacy compatibility does not raise `CarrierLimits::default()` or weaken explicit bounded calls.
- JPEG bytes, seeds, redundancy selection, capacity units, support classification, and coefficient decode implementation are unchanged.
- Legacy parser-limit failures map to `MalformedInput`; explicit configured budget exhaustion maps to `ResourceLimitExceeded`.
- JPEG dimensions are bounded by the structural SOF maximum in the compatibility profile; checked product derivation fits the JPEG field domain.
- Root input, segment, and dimension limits remain independent and are checked before its canonical carrier work.
- M005 WebP remains explicitly bounded. `#![forbid(unsafe_code)]` remains enabled.

## 6. Failure and recovery review

Bounded checks still fail before embedding output is produced. Compatibility calls remain synchronous and do not add retries or shared mutable budgets. Prepared handles retain decoded state after failed operations. Frame size and tiled-origin failures remain bounded by their preexisting structural maximums.

## 7. Migration and compatibility review

No caller migration is needed. The default carrier limits and explicit bounded API semantics did not change. Legacy calls regain the pre-M003 effective structural domain. Public one-shot operations first exposed after v0.4.2 are routed consistently through the compatibility profile; no published v0.4.2 API contract is claimed for those later additions.

## 8. Security review

The direct generic carrier compatibility surface retains preexisting segment/frame/tiled bounds but has no whole-input or lower dimension policy ceiling, matching its historical contract. Untrusted-input consumers should use explicit bounded variants. The root application continues to apply `ResourceLimits` before using the compatibility carrier paths. No secret or input bytes are exposed in errors.

## 9. Documentation and operations

Updated `stegoeggo-stego/README.md`, JPEG and `PreparedJpeg` rustdoc, `docs/carrier-crate.md`, `STABILITY.md`, `architecture/resource-limits.md`, `architecture/carrier-jpeg.md`, `CHANGELOG.md`, subsystem roadmap, registry, and this closure record. No release was published; the latest published release remained v0.4.2 during implementation.

## 10. Unresolved findings (critical/high/medium/low)

None. M006 remains blocked on ADR-0007 acceptance; M007 remains proposed pending an explicit maintainer authorization decision.

## 11. Roadmap disposition

M008 closed. M006's M008 dependency is satisfied; its sole remaining blocker is acceptance of ADR-0007. M007 has no implementation handoff and remains proposed until a maintainer explicitly authorizes the algorithm and threat-model expansion. The workstream's compatibility discrepancy is resolved before the next carrier release.

## 12. Registry updates

`plans/registry.md` removes M008 from ready implementation work, records it as recently closed, and changes M006's blocker to ADR-0007 acceptance only. `plans/subsystems/stego-library-evolution-roadmap.md` marks M008 closed, updates the current state and dependency narrative, and removes M008 closure as a remaining blocker for M006/M007. The implementation plan status is closed.
