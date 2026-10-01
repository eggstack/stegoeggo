# Verification and Conformance Milestone 006 — Unstructured Candidate False-Positive Corrective

Status: ready for handoff

Repository baseline: `f80eebe3fb983a5e8b13f7e082fe573636d65756`

Source roadmap:
`plans/subsystems/verification-conformance-roadmap.md#m6--unstructured-candidate-false-positive-corrective`

Corrects post-closure regression introduced after the accepted verification
model convergence work and observed by the Python/Node compatibility suites.

Long-term requirements:

- `plans/000-long-term-specification.md#2-canonical-api-invariants`
- `plans/001-terminology-and-domain-model.md#verification-model`
- `plans/002-long-term-roadmap.md#phase-0--foundation`

Applicable ADRs:

- `plans/adrs/ADR-0001-canonical-protection-request.md`
- `plans/adrs/ADR-0003-byte-vs-pixel-paths.md`

Primary class: invariant

## 1. Objective

Restore the canonical verification invariant that a valid image with no
credible StegoEggo hidden-marker evidence reports hidden-marker
`VerificationStatus::NotFound`, while preserving `Invalid` for a candidate
that has enough StegoEggo structure/provenance to represent an actually
corrupted, malformed, unsupported, or authentication-failed marker.

The corrective must fix the behavior in canonical Rust verification rather
than teaching Python/Node wrappers to reinterpret `Invalid` as `NotFound`.

It must also add Rust-level regression coverage so this property no longer
depends on foreign-binding CI to detect breakage.

## 2. Why this milestone is ready

The failure is reproduced by two independent foreign frontends on the same
canonical API and is not a binding build problem.

On commit `8c89e8cb1677a355d639ca1ead93c3dd2587e317`:

- standard Rust CI run `36874105954` succeeded;
- Node binding run `36874106064` built successfully and generated-file
  drift was clean, then failed four behavioral assertions because ordinary
  inputs returned hidden-marker status `INVALID` instead of `NOT_FOUND`;
- Python binding run `36874106092` independently failed
  `test_verify_unprotected_image_has_no_rights` for the same
  `Invalid` versus `NotFound` discrepancy.

The triggering correctness commit removed structural-plausibility suppression
from unstructured/fallback candidate probes in
`src/protected/steganography/verify.rs`. The old suppression should not be
blindly restored because it contained seed/probe-specific heuristics. The
corrective therefore needs an explicit candidate-evidence rule.

No architecture ADR is required: the accepted verification model already
distinguishes `Invalid` from `NotFound`.

## 3. Current implementation evidence

Canonical verification currently performs:

1. rights metadata extraction;
2. hidden-marker candidate search through
   `SteganographyProtector::verify_payload_from_bytes_outcome`;
3. projection from `CandidateOutcome` to `CanonicalOutcomeKind` and
   `VerificationStatus`.

`CandidateOutcome::Invalid` is intended to mean that a plausible payload was
found but integrity/authentication failed. The module-level documentation says
this distinction exists so corruption can surface as `Invalid` rather than
being flattened into `NotFound`.

The regression is that unstructured fallback extraction can now produce
`Invalid` from ordinary carrier bytes without first proving that the bytes
represent a plausible StegoEggo payload candidate. This makes absence look
like corruption.

Observed Node failures on run `36874106064`:

- unprotected image: actual `INVALID`, expected `NOT_FOUND`;
- metadata-only image hidden marker: actual `INVALID`, expected
  `NOT_FOUND`;
- concurrent metadata-only operations: same false-positive status;
- unspecified policy / empty notice result: same false-positive status.

The Python run independently reproduces the unprotected-image failure.

## 4. Invariants that must not regress

- `verify_image_bytes_report` remains the canonical rich verification
  operation.
- No credible marker evidence on a valid carrier means hidden-marker
  `NotFound`, not `Invalid`.
- `Invalid` remains observable for actual structured marker corruption.
- Wrong HMAC key remains authentication failure/invalid, never `NotFound`.
- Missing required HMAC key remains distinguishable from wrong key.
- Malformed V3 and unsupported-version candidates remain diagnosable once V3
  structure/magic establishes marker provenance.
- Resource-limit exhaustion remains distinguishable from ordinary absence.
- Explicit metadata/Q-table seed evidence must not be discarded merely because
  later payload verification fails.
- Legacy V1/V2 compatibility remains supported.
- Valid seed value zero remains valid; the corrective must not reintroduce a
  special rule that treats seed 0 as absence.
- Verification budgets/resource limits stay bounded.
- No extra carrier scan pass is introduced if the existing candidate result can
  carry sufficient provenance.
- Existing root/carrier `#![forbid(unsafe_code)]` policy remains unchanged.
- Foreign bindings consume the Rust result; Rust must not depend on binding
  behavior.

## 5. Scope

### In scope

- `src/protected/steganography/verify.rs` candidate classification.
- Minimal supporting changes to candidate outcome/provenance types if needed.
- `src/verification/canonical.rs` only where projection needs to consume the
  corrected distinction.
- Rust regression tests for:
  - ordinary unprotected PNG/JPEG/WebP;
  - metadata-only protected images;
  - random/unstructured fallback candidates;
  - structured corrupted candidates;
  - HMAC wrong/missing/correct-key paths;
  - seed 0;
  - malformed/unsupported V3;
  - resource-limit outcomes.
- Existing verification convergence/report tests as regression evidence.
- Documentation comments needed to describe the corrected classification.

### Explicitly out of scope

- Changing Python or Node wrappers/tests.
- Redefining the public verification model.
- Adding new `VerificationStatus` values.
- Removing legacy payload support.
- Weakening corruption/authentication reporting to make tests green.
- Increasing verification budgets to avoid classification problems.
- New stego algorithms or container formats.
- C ABI work.
- Release artifact publication.

## 6. Required production changes

### A. Make candidate provenance explicit

Do not rely on “extracted some bytes” as proof that a marker exists.

The implementation must establish, either in `CandidateOutcome` or in a
small internal classifier, whether the candidate arose from evidence strong
enough to treat integrity failure as `Invalid`.

At minimum distinguish:

1. **explicit/provenance-bearing candidate**
   - metadata seed tied to StegoEggo-owned metadata;
   - JPEG Q-table seed/hint;
   - recognized V3 magic/header prefix;
   - parseable V1/V2 payload structure;
   - other existing marker framing that uniquely identifies StegoEggo;
2. **unstructured probe**
   - fixed-position/fallback seed guessed from ordinary pixel bits;
   - configured/test fallback-seed search without independent marker evidence;
   - arbitrary extraction bytes that do not parse as supported StegoEggo
     structure.

An unstructured probe that fails parsing/integrity is `NotFound`.
A provenance-bearing candidate that fails integrity remains the appropriate
invalid/malformed/authentication outcome.

The exact type layout is implementation-local. Prefer a small explicit enum or
classifier over several seed-value booleans.

### B. Remove seed-value heuristics

Do not restore the historical `fallback_seed == 0` special case.

Seed 0 is a valid documented seed. Candidate confidence must derive from the
source/structure of the evidence, not the numeric seed value.

### C. Preserve V3 diagnostics

Recognized V3 magic/header evidence must continue to surface:

- malformed header;
- unsupported version;
- authentication key missing;
- authentication failed;
- integrity failure;
- resource-limit exhaustion.

The corrective must not map a recognized V3 prefix to `NotFound` merely
because full payload parsing later fails.

### D. Preserve legacy corruption reporting

For V1/V2, only report `Invalid` when the extracted bytes are structurally
credible as the corresponding supported legacy payload. Random carrier bytes
from an unstructured probe are absence.

Use the existing parsing/framing rules as the oracle; do not invent a looser
magic heuristic.

### E. Add a canonical absence guard

Add a Rust test at the canonical verification boundary, not only at the
private steganography helper, proving that each supported valid unprotected
container yields:

- `rights_found == false` when the fixture has no StegoEggo rights;
- hidden marker status `NotFound`;
- no corruption/authentication diagnostic invented from random pixels.

Also prove a metadata-only image has rights evidence while its hidden marker
remains `NotFound`.

## 7. Ordered work packages

### WP1 — Reproduce and freeze the regression in Rust

Add failing tests against current `main` using the same canonical fixture
class that Python/Node exposed.

Acceptance:

- tests fail on the current broken classifier for the same reason as Actions;
- tests are at the canonical Rust boundary;
- PNG is mandatory; JPEG/WebP are included where fixture behavior is
  deterministic.

### WP2 — Introduce explicit candidate-confidence classification

Refactor the private verification path so candidate outcome is interpreted in
the context of how the candidate was discovered.

Acceptance:

- no seed-value heuristic;
- no unstructured random candidate can become `Invalid`;
- explicit marker provenance still reaches the detailed invalid variants.

### WP3 — Corruption/authentication regression matrix

Exercise real marker evidence with targeted corruption.

Acceptance:

- correct marker/key => `Verified`;
- wrong HMAC key => invalid/auth failed;
- missing HMAC key => key-missing behavior;
- structured CRC/integrity corruption => `Invalid`;
- malformed/unsupported V3 remains diagnostic;
- unstructured absence => `NotFound`.

### WP4 — Resource-limit and seed-zero qualification

Acceptance:

- resource-limit exhaustion is not flattened to `NotFound`;
- seed 0 protects/verifies correctly;
- fallback search remains bounded by existing limits.

### WP5 — Full canonical regression verification

Run root tests and the targeted verification suites.

Acceptance:

- `./scripts/check.sh` green;
- targeted verification tests green;
- no existing accepted convergence/report invariant regresses.

## 8. Failure, cancellation, restart, and contention semantics

- If a candidate is ambiguous and there is no StegoEggo-specific structural
  evidence, fail toward `NotFound`, not corruption.
- If an explicit StegoEggo prefix/seed source exists but processing is
  resource-limited, preserve the resource-limit condition.
- Do not retry extraction with unbounded seeds/origins after a failed probe.
- A failed candidate must not contaminate state used by later candidate probes.
- Candidate ordering must not change the final classification for equivalent
  evidence.
- If two candidates exist, a later valid candidate may establish
  `Verified`; an earlier unstructured false candidate must not terminate the
  search as `Invalid`.
- If implementing provenance requires a broader public API change, stop and
  report rather than leaking a new compatibility contract from a corrective.

## 9. Compatibility and migration

The intended behavior is a regression correction, not a new public semantic:

- ordinary unprotected images return to `NotFound`;
- metadata-only protection continues to report rights with no hidden marker;
- genuinely corrupted recognized markers remain `Invalid`.

No serialized report schema change is expected. Internal enums may change.

If existing Rust tests intentionally relied on arbitrary unstructured bytes
surfacing `Invalid`, identify those tests and reconcile them against the
accepted verification model rather than silently changing expectations.

## 10. Required tests

At minimum add or retain tests for:

1. unprotected canonical PNG => `NotFound`;
2. unprotected JPEG => `NotFound`;
3. unprotected WebP => `NotFound`;
4. metadata-only PNG => rights found + marker `NotFound`;
5. metadata-only JPEG/WebP equivalent where supported;
6. random fallback-seed bytes do not produce `Invalid`;
7. seed 0 valid marker verifies;
8. deterministic nonzero seed verifies;
9. V1 corruption remains invalid;
10. V2 corruption remains invalid;
11. V3 structured checksum/integrity corruption remains invalid;
12. malformed V3 prefix remains diagnostic;
13. unsupported V3 version remains diagnostic;
14. correct HMAC key verifies;
15. wrong HMAC key remains invalid/auth failed;
16. missing key remains distinguishable;
17. resource-limit candidate remains resource-limited/invalid as designed;
18. candidate search can continue past an unstructured false probe to a later
    valid candidate where the algorithm supports multiple probes;
19. canonical `VerificationResult` projection remains consistent with the
    report.

## 11. Required verification commands

Minimum:

```bash
cargo test --workspace --exclude stegoeggo-fuzz verification
cargo test --workspace --exclude stegoeggo-fuzz --test verification_convergence
cargo test --workspace --exclude stegoeggo-fuzz --test verification_report_tests
./scripts/check.sh
```

If exact integration-test binary names differ, record the actual equivalent
commands in closure rather than skipping them.

After the core fix lands, use the binding workflows as downstream acceptance
signals but do not close this milestone solely from them:

- `python-binding.yml`;
- `node-binding.yml`.

Both must at least pass the unprotected-image verification assertions before
language-bindings corrective closure can complete.

## 12. Documentation updates

Update:

- `architecture/verification.md` only if current wording does not already
  capture the absence-versus-corruption distinction;
- `plans/subsystems/verification-conformance-roadmap.md`;
- `plans/registry.md`;
- closure record `plans/closure/verification-conformance/006-status.md`.

Do not rewrite prior flat plans or their accepted closure evidence.

## 13. Acceptance criteria

M006 closes only when:

- canonical Rust verification reports `NotFound` for ordinary valid
  unprotected images;
- metadata-only images do not invent a hidden-marker `Invalid`;
- real structured corruption still reports `Invalid`;
- wrong/missing HMAC-key behavior remains distinct;
- V3 malformed/unsupported behavior remains diagnostic;
- seed 0 remains valid;
- verification/resource budgets remain enforced;
- Rust-level regression tests cover the exact false-positive class;
- `./scripts/check.sh` is green;
- the downstream Python and Node unprotected-image tests no longer fail for
  `Invalid` versus `NotFound`.

## 14. Stop conditions

Stop and report if:

- fixing absence requires treating all integrity failures as `NotFound`;
- candidate provenance cannot be represented without changing the public
  verification/report schema;
- a fix would remove support for valid seed 0;
- a fix would require unbounded fallback scanning;
- a fix would suppress recognized malformed/unsupported V3 evidence;
- a fix requires changing established authentication semantics;
- the Python/Node failures reveal that the fixture actually contains
  StegoEggo marker evidence and the current expected `NotFound` is wrong.

## 15. Closure evidence required

Create `plans/closure/verification-conformance/006-status.md` and record:

- implementation commit(s);
- exact root baseline;
- before/after result for the unprotected canonical fixture;
- candidate-confidence design used;
- seed-zero result;
- structured corruption matrix;
- HMAC correct/wrong/missing results;
- malformed/unsupported V3 results;
- resource-limit result;
- exact Rust test commands and counts;
- `./scripts/check.sh` result;
- downstream Python/Node run IDs showing the false-positive status regression
  is gone, even if another independent binding corrective remains;
- any intentionally changed internal behavior;
- roadmap/registry disposition.

## 16. Handoff notes

Start from the failing foreign-binding evidence, but fix Rust first.

Do not simply restore the deleted pre-`8c89e8cb` blocks. Their
seed-specific suppression is evidence about the previous failure mode, not the
desired architecture.

The useful invariant is: **absence is not corruption**. A candidate needs
StegoEggo-specific provenance/structure before an integrity failure can become
`Invalid`.

The language-bindings M006 corrective may proceed in parallel against this
written contract, but it cannot close until this verification milestone is
accepted.
