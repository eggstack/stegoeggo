# Stego Library Evolution Milestone 008 — M003 Default-Limit Compatibility Corrective

Status: ready for handoff  
Repository baseline: `0a8b356a056bdab2f9d1996be78578cb3f6bc724`  
Source roadmap: `plans/subsystems/stego-library-evolution-roadmap.md#7-milestones`  
Corrects: `plans/implementation/stego-library-evolution/003-carrier-resource-and-prepared-hardening.md` and `plans/closure/stego-library-evolution/003-status.md`  
Long-term requirements: `plans/000-long-term-specification.md#2-canonical-api-invariants`, `#3-execution-invariants`  
Applicable ADRs: `plans/adrs/ADR-0002-output-domain-carrier-routing.md`  
Primary class: invariant

## 1. Objective

Correct the behavioral-compatibility regression introduced by M003 when pre-existing
generic JPEG entry points were routed through `CarrierLimits::default()`.

Preserve the M003 bounded API as the explicit untrusted-input interface while restoring
the effective pre-M003 acceptance/error contract of the stable 0.x one-shot JPEG APIs
and `PreparedJpeg::new` before any post-0.4.2 carrier release.

This corrective does not remove `CarrierLimits`, weaken explicit `*_with_limits`
operations, or reopen M004/M005.

## 2. Why this milestone is ready

M003–M005 are closed and the latest public GitHub release remains v0.4.2, published
before `CarrierLimits` landed. The compatibility defect therefore exists on `main`
but has not yet been published as a newer stable release.

The corrective is independent of ADR-0007. It should close before M006 or any next
carrier release so keyed-placement work does not build on an ambiguous legacy/default
resource contract.

## 3. Current implementation evidence

Before M003 at baseline `d5425d3`:

- `jpeg::inspect(bytes, max_segments, max_segment_bytes)` bounded JPEG segment count
  and segment length exactly as supplied by the caller, but imposed no 100 MiB
  whole-input cap and no 16,384-pixel dimension cap.
- The private `decode_supported_carrier` called the transcoder's existing default
  parser limits: 256 JPEG segments and 65,535 bytes per segment. Parser failures were
  mapped to `StegoError::MalformedInput`.
- `probe_support`, `capacity`, `embed`/strict/framed/tiled variants,
  `extract` variants, and `PreparedJpeg::new` did not reject solely because encoded
  input exceeded 100 MiB or one dimension exceeded 16,384.
- JPEG SOF dimensions are structurally limited by the format's 16-bit width/height
  fields; the previous code therefore had a structural maximum rather than M003's
  lower policy maximum.

After M003 at `6d87c57` and current `0a8b356a056bdab2f9d1996be78578cb3f6bc724`:

- `CarrierLimits::default()` is 100 MiB input, 16,384×16,384 dimensions/pixel
  envelope, 256 JPEG segments, 65,535 bytes per segment, 16 MiB+11 framed bytes, and
  4,096 tiled origins.
- Pre-existing one-shot JPEG functions delegate to
  `decode_supported_carrier_with_limits(..., CarrierLimits::default())`.
- `probe_support` and `inspect` also apply the new input/dimension limits.
- Parse-limit errors containing `"exceeds limit"` now map to
  `StegoError::ResourceLimitExceeded`; pre-M003 one-shot decode paths mapped those
  transcoder failures to `MalformedInput`.
- `PreparedJpeg::new` predates M003 but now inherits the new default policy bounds.
  `PreparedJpeg::new_with_limits` is the new bounded constructor and is not a
  compatibility problem.
- M005 WebP APIs were introduced after `CarrierLimits` and explicitly require/use the
  bounded contract; they have no pre-M003 compatibility surface to restore.

Why M003 verification missed this: semver-checks verifies public API shape, not accepted
input domains or error variants for unchanged signatures. M003 parity tests exercised
ordinary inputs under the new defaults and custom tighter bounds, but did not include a
valid >16,384-dimension/low-pixel JPEG or pre/post error-precedence fixtures.

## 4. Invariants that must not regress

- Every API that existed in v0.4.2 keeps its established successful-input domain except
  for pre-existing parser/frame/tiled structural bounds.
- Existing seed, redundancy, carrier bytes, capacity units, output bytes, and JPEG
  support classification remain unchanged.
- The explicit M003 `*_with_limits`, `inspect_with_limits`, and
  `PreparedJpeg::new_with_limits` operations retain bounded semantics and
  `ResourceLimitExceeded` classification.
- `CarrierLimits::default()` remains a useful conservative configuration for new code;
  do not inflate its security-oriented defaults merely to emulate legacy behavior.
- Pre-M003 JPEG parser limits remain 256 segments / 65,535 bytes per segment.
- Existing frame maximum and `MAX_TILED_ORIGINS` behavior remain unchanged.
- Root StegoEggo resource policy remains bounded by its own `ResourceLimits`; restoring
  direct-carrier compatibility must not bypass root/application limits.
- M005 lossless-WebP APIs remain explicitly bounded.
- `#![forbid(unsafe_code)]` remains true.

## 5. Scope

In:

- Classify every public JPEG/`PreparedJpeg` operation as either pre-M003 compatibility
  surface or M003+ bounded surface.
- Add a private compatibility limit/profile for legacy one-shot delegation.
- Restore pre-M003 input/dimension acceptance for legacy one-shot functions without
  changing `CarrierLimits::default()`.
- Restore pre-M003 `MalformedInput` classification for parser-limit failures reached
  through legacy one-shot APIs where that was the prior behavior.
- Preserve `ResourceLimitExceeded` for explicit bounded APIs.
- Add regression fixtures crossing the 16,384 dimension threshold without large memory
  demand.
- Reconcile carrier docs, stability text, architecture, roadmap language, and M003
  historical claims through this corrective record rather than rewriting M003 closure.

Out:

- Removing `CarrierLimits` or any `*_with_limits` API.
- Removing all safety bounds from the generic crate.
- Changing root `ResourceLimits`.
- Changing WebP limits.
- Publishing a release.
- Accepting ADR-0007 or implementing keyed placement.
- Editing the historical M003 closure to make the corrective appear unnecessary.

## 6. Required production changes

### 6.1 Define a private legacy-compatibility profile

Add one private/internal compatibility profile used only by APIs that existed before
M003. Prefer a helper such as `legacy_compatibility_limits()`; do not expose a public
"unbounded" constructor.

The profile MUST preserve:

- JPEG segment count: 256.
- JPEG segment bytes: 65,535.
- frame maximum: the existing frame codec maximum.
- tiled origins: `MAX_TILED_ORIGINS`.

For M003-added whole-input/dimension fields, use the pre-M003 structural domain rather
than the new policy defaults:

- encoded input: no additional policy cap beyond addressable input (`usize::MAX`);
- width/height: JPEG structural maximum representable by SOF (`u16::MAX`);
- pixel-count policy: no lower arbitrary cap than the structural width×height domain.

Use checked arithmetic when deriving the structural pixel maximum.

### 6.2 Split compatibility and explicitly bounded error mapping

Do not use one string-matching error mapper to force both surfaces into one error
contract.

- Legacy one-shot APIs preserve the pre-M003 mapping of transcoder/parser-limit
  failures to `StegoError::MalformedInput`.
- Explicit `*_with_limits` APIs preserve M003's
  `StegoError::ResourceLimitExceeded` classification for caller/configured budget
  exhaustion.
- Unsupported-JPEG classification remains unchanged.

Prefer an internal decode mode/helper or typed internal limit error mapping over
duplicating the coefficient decoder.

### 6.3 Route the complete pre-M003 surface through compatibility semantics

Audit at minimum:

- `jpeg::inspect`
- `jpeg::probe_support`
- `jpeg::capacity`
- `jpeg::embed`
- `jpeg::embed_best_effort` only if it delegates from the pre-existing
  `embed` compatibility operation; its own explicit M002 behavior must remain
  byte-identical
- `jpeg::embed_strict`
- framed embed variants that existed before M003
- `jpeg::extract` and framed extraction
- tiled embed/extract variants that existed before M003
- `PreparedJpeg::new`

M003-added `*_with_limits`, `inspect_with_limits`,
`probe_support_with_limits`, and `PreparedJpeg::new_with_limits` remain on
`CarrierLimits::default()`/caller-provided bounded semantics.

If API-history audit shows any item above did not exist in v0.4.2, classify it from Git
history rather than assuming.

### 6.4 Keep parent policy bounded

Verify the root application still applies its own `ResourceLimits` before or while
entering carrier work. If any canonical root path currently relies only on the
carrier's 100 MiB/16,384 defaults, translate the root limits explicitly into
`CarrierLimits` and call the bounded variants instead of relying on legacy wrappers.

Do not weaken root DoS hardening to repair direct-crate compatibility.

## 7. Ordered work packages

### WP1 — API-history and behavioral matrix

Create a checked matrix of every current JPEG/`PreparedJpeg` public operation with:

- present in v0.4.2? yes/no;
- pre-M003 parse/input/dimension bound;
- pre-M003 error variant on segment-limit exhaustion;
- current M003 behavior;
- required post-corrective behavior.

Acceptance: no operation is routed by naming convention alone.

### WP2 — Compatibility profile and decode/error split

Implement the private legacy profile and shared decode plumbing. Keep one coefficient
decoder and one support classifier; parameterize policy/error mapping rather than fork
the codec.

Acceptance: pre-existing entry points no longer inherit 100 MiB/16,384 policy caps,
while explicit bounded variants are byte/error compatible with M003.

### WP3 — Boundary regression fixtures

Add a valid supported JPEG with width >16,384 and small height (for example 20,000×1
or another encoder-supported low-pixel fixture) so memory use stays small.

Prove:

1. legacy `probe_support`, `capacity`, and `PreparedJpeg::new` reach their
   pre-M003 success/support path;
2. the corresponding explicit `*_with_limits` call using
   `CarrierLimits::default()` returns `ResourceLimitExceeded`;
3. `inspect` with permissive caller segment limits preserves legacy acceptance while
   `inspect_with_limits(default)` enforces the dimension policy;
4. a parser segment-limit fixture returns the historical error variant through the
   legacy surface and `ResourceLimitExceeded` through explicit bounded surface.

Do not add a routine 100+ MiB CI fixture. Pin the compatibility input cap through unit
tests on the private profile and, if useful, an ignored/manual large-input regression.

### WP4 — Root/application bound audit

Exercise canonical request/verification paths with configured tight limits and prove
they still fail at the root policy boundary after legacy direct-carrier behavior is
restored.

Acceptance: direct-crate compatibility restoration does not weaken StegoEggo's
application-level resource hardening.

### WP5 — Documentation and planning reconciliation

Update carrier rustdoc/README, `docs/carrier-crate.md`, `STABILITY.md`,
`architecture/resource-limits.md`, `architecture/carrier-jpeg.md`, and any example
that implies all old one-shot calls use conservative defaults.

Document two contracts plainly:

- legacy 0.x one-shot APIs preserve the v0.4.2 effective domain;
- explicit `*_with_limits` / `new_with_limits` APIs are the recommended
  untrusted-input interface.

Do not rewrite M003 closure. M008 closure records the discovered behavioral gap and its
correction.

## 8. Failure, cancellation, restart, contention semantics

No mutation/output may occur after a bounded failure. Legacy compatibility paths retain
the same synchronous behavior they had before M003; restoring their historical domain
does not create a retry loop or global resource budget.

The root application must continue failing before dangerous work when its configured
limits are exceeded.

## 9. Compatibility and migration

No caller migration is required for v0.4.2 code.

Existing one-shot callers regain the historical acceptance/error behavior they would
expect from an unchanged 0.x signature. New or network-facing generic callers should use
`CarrierLimits` plus explicit bounded variants.

Because M003–M005 have not been published after v0.4.2, this corrective should land
before the next stable carrier/library/CLI version and needs no published-version
rollback.

## 10. Required tests

At minimum:

- API-history/compile coverage for all pre-M003 one-shot functions.
- Valid wide-but-low-pixel supported JPEG (>16,384 width) legacy-vs-bounded tests.
- `PreparedJpeg::new` vs `new_with_limits` boundary test.
- `inspect` vs `inspect_with_limits` dimension and segment-error classification.
- Legacy one-shot parse-limit `MalformedInput` regression.
- Explicit bounded `ResourceLimitExceeded` regression.
- Existing default-parity tests for ordinary inputs.
- Known-answer vectors.
- M003 adversarial carrier-limit tests.
- M005 WebP bounded tests.
- Root `ResourceLimits` regression demonstrating application policy remains enforced.

## 11. Required verification commands

```bash
cargo test -p stegoeggo-stego --all-features
cargo test --workspace --all-features --test public_stego_api
cargo test --workspace --all-features --test known_answer_vectors
cargo test --workspace --all-features --test robustness
./scripts/check.sh
cargo semver-checks check-release -p stegoeggo-stego
```

Also compare the affected behavior against a v0.4.2 worktree or fixture harness. Record
the exact command and result in closure; semver-checks alone is insufficient evidence
for this milestone.

## 12. Documentation updates

Required:

- `stegoeggo-stego/README.md`
- JPEG and `PreparedJpeg` rustdoc
- `docs/carrier-crate.md`
- `STABILITY.md`
- `architecture/resource-limits.md`
- `architecture/carrier-jpeg.md`
- `plans/subsystems/stego-library-evolution-roadmap.md`
- `plans/registry.md` at closure

Update `CHANGELOG.md` if M003/M005 additions are already described in Unreleased.

## 13. Acceptance criteria

M008 may close only when:

1. every public operation that existed in v0.4.2 has an explicit compatibility
   disposition;
2. legacy one-shot JPEG APIs do not reject solely because input exceeds M003's new
   100 MiB or 16,384 policy thresholds;
3. pre-existing parser-limit error classification is restored where M003 changed it;
4. explicit bounded APIs still enforce `CarrierLimits::default()` exactly;
5. root/application resource limits remain enforced;
6. known-answer and output bytes are unchanged;
7. WebP bounded behavior is unchanged;
8. v0.4.2 behavioral comparison evidence and `./scripts/check.sh` are green.

## 14. Stop conditions

Stop and report rather than improvise if:

- restoring a v0.4.2 behavior requires exposing codec internals;
- the old successful-input domain cannot be preserved without a demonstrated
  unavoidable memory-safety/correctness failure;
- root DoS protection depends on the legacy wrappers and cannot be moved to explicit
  bounded calls without changing public semantics;
- a public API believed to predate M003 is absent from v0.4.2;
- the corrective would require changing carrier bytes, seed mapping, or frame format.

Any unavoidable intentional narrowing of a v0.4.2 accepted-input domain requires a
separate durable compatibility decision; do not silently retain the M003 behavior.

## 15. Closure evidence required

Record:

- implementation SHA(s);
- complete API-history/behavior matrix;
- v0.4.2 comparison worktree/harness command and results;
- wide-JPEG fixture dimensions/size and legacy-vs-bounded outcomes;
- parser-limit error-variant comparison;
- root resource-limit evidence;
- focused suite counts;
- semver-checks result with the explicit note that it does not establish behavioral
  compatibility;
- `./scripts/check.sh` result;
- confirmation that latest published release remained v0.4.2 while the correction was
  made.

## 16. Handoff notes

This is a compatibility corrective, not an argument against bounded APIs. The intended
end state is two honest contracts: compatibility entry points preserve 0.x behavior;
explicit bounded entry points are the safe choice for untrusted inputs.

Do not solve the issue by raising `CarrierLimits::default()` globally. That would make
the bounded API less useful and hide the distinction this corrective is meant to make.
