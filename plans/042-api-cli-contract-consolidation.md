# Plan 042: API and CLI Contract Consolidation

Status: Ready for implementation

Baseline: `main` after Plan 041 planning commit `a5cd43f05ea08e44b5311d0d9a11ab1f8c01e3e7`

Depends on:

- `plans/038-rights-metadata-format-correctness-roadmap.md`
- `plans/039-plus-iptc-rights-metadata-correctness.md`
- `plans/040-jpeg-dct-container-correctness-and-containment.md`
- `plans/041-webp-container-xmp-exif-correctness.md`

Must be completed before:

- `plans/043-measured-binary-size-reduction.md`
- `plans/044-cross-format-correctness-closure.md`

---

## Purpose

Collapse StegoEggo's duplicated public and CLI policy paths into one canonical request/plan execution model, and correct API contracts that imply file-level metadata can survive a `DynamicImage` round trip.

The repository currently has:

- `ProtectionRequest` and `ResolvedProtectionPlan`, described as the canonical API;
- legacy `ProtectionLevel`, `ProtectionContext`, and `EvidenceProfile` behavior;
- separate legacy and request-based CLI branches;
- option combinations that select one branch and silently discard values assembled for the other;
- verify-mode key handling that does not consistently use the documented key-source resolver;
- a `ProtectionPipeline::process(&DynamicImage, ...)` API that encodes metadata into bytes and then decodes those bytes back into pixels, necessarily losing the primary file-level metadata channel;
- production CLI dependency on the `test-seeds` feature.

This plan preserves compatibility where practical but removes independent policy decisions. One request is resolved once and executed once.

---

## Governing decisions

1. `ProtectionRequest` is the canonical caller intent model.
2. `ResolvedProtectionPlan` is the canonical validated execution model.
3. Legacy APIs translate into a request; they do not run separate policy logic.
4. The CLI translates arguments into one request regardless of which syntax the user chooses.
5. Conflicting legacy/new options are rejected rather than resolved by branch-order accidents.
6. Byte APIs are the complete rights-metadata APIs.
7. Pixel APIs are explicitly pixel-only and cannot claim metadata injection.
8. Key-source behavior is consistent across protect, verify, sign, and manifest operations where applicable.
9. Test/development seed guessing is not part of the normal installed CLI.
10. This plan does not require a breaking release or publication.

---

## Required end state

### Library

- equivalent legacy and request inputs resolve to equivalent plans;
- validation occurs once in `resolve_request` or one shared resolver;
- deprecated types remain adapters or aliases, not alternate policy engines;
- byte APIs expose complete metadata + optional hidden-marker behavior;
- pixel-only APIs are named/documented so callers cannot mistake them for file metadata operations;
- execution reports reflect format fallback and channel outcomes established by Plans 039-041.

### CLI

- one argument-normalization function produces one `ProtectionRequest` plus operation mode;
- one processing path handles single and batch protection;
- verify mode uses the same key resolver as protect mode;
- mixed conflicting options fail with exit code 2/configuration error;
- JSON and human-readable output derive from the same execution report;
- `--dry-run` resolves the exact request that normal execution would use;
- `test-seeds` is not enabled by the normal CLI dependency.

---

## Non-goals

Do not use this plan to:

- redesign the rights policies established by Plan 039;
- reopen JPEG/WebP mechanics;
- remove all deprecated APIs immediately;
- introduce a command framework beyond Clap;
- add config files, profiles on disk, or environment-wide policy engines;
- add a daemon or network API;
- redesign detached manifests or signatures;
- add a generalized secret manager;
- add required CI jobs;
- publish a release.

---

## Phase 0: Establish an API/CLI behavior ledger

Create `plans/042-status.md` before source edits.

Initialize it with:

```text
Plan baseline SHA: a5cd43f05ea08e44b5311d0d9a11ab1f8c01e3e7
Disposition: OPEN
Canonical request path: OPEN
Legacy adapter equivalence: OPEN
CLI single path: OPEN
Mixed-option validation: OPEN
DynamicImage contract: OPEN
Key-source consistency: OPEN
Test-seeds production removal: OPEN
Documentation: OPEN
Publication hold: no publication is part of this plan
```

Add these tables.

### Table A: public entry points

```text
entry point | current model | returns bytes/pixels | metadata survives | canonical/deprecated | target disposition | status
```

Include at least:

- `ProtectionPipeline::process`;
- `ProtectionPipeline::process_bytes`;
- `process_image`;
- `process_image_bytes`;
- request-based byte functions;
- warning/report variants;
- async wrappers;
- parallel wrappers;
- verification functions.

### Table B: CLI option interactions

```text
options | current branch | current effective policy/channels | required behavior | test | status
```

Required combinations:

- no new-style flags;
- `--preset legal-notice`;
- `--preset legal-notice-with-stego`;
- `--rights-policy prohibited-ai-ml-training`;
- `--preset ... --no-ai-training`;
- `--rights-policy allowed --no-ai-training`;
- `--dmi ... --rights-policy ...`;
- `--level light --preset maximal`;
- `--authentication hmac` without a key;
- `--hidden-marker disabled --authentication hmac`;
- `--no-metadata` with legal fields;
- `--dry-run` with each syntax family.

### Table C: key sources

```text
operation | literal hex | @file | stdin | environment | current | target | status
```

### Phase 0 acceptance criteria

- every public processing entry point is classified;
- every ambiguous CLI combination is assigned a target behavior;
- compatibility commitments are explicit;
- no behavior is marked complete without tests;
- the implementation agent does not begin by deleting deprecated APIs blindly.

Suggested commit:

```text
plans: inventory API and CLI contract duplication
```

---

## Phase 1: Define one internal operation model

### 1.1 Canonical operation enum

The CLI may need more than a request because verify/keygen/sign/manifest commands are distinct operations.

Use a small internal model such as:

```rust
enum CliOperation {
    Protect {
        inputs: Vec<PathBuf>,
        output: Option<PathBuf>,
        request: ProtectionRequest,
        jobs: usize,
        strict: bool,
    },
    Verify {
        input: PathBuf,
        key: Option<Vec<u8>>,
    },
    DryRun {
        input: PathBuf,
        request: ProtectionRequest,
    },
    #[cfg(feature = "signatures")]
    Keygen { /* ... */ },
    // manifest operations
}
```

The exact type may differ. Keep it local to the CLI.

### 1.2 One request builder

Create one function:

```rust
fn build_protection_request(args: &Args) -> Result<ProtectionRequest, CliConfigError>
```

It must:

- normalize legal metadata;
- normalize policy shorthands;
- normalize preset/level/profile/channel options;
- normalize processing options;
- resolve key input once;
- detect conflicts;
- produce the exact request used by dry-run and execution.

Do not let `build_legal_metadata()` return a DMI override that one branch ignores.

### 1.3 Explicit precedence versus conflict

Use explicit rules.

Recommended policy:

- shorthand flags such as `--no-ai-training` are aliases for a rights policy only when no explicit `--rights-policy` conflicts;
- equivalent repeated expressions are allowed;
- contradictory expressions are configuration errors;
- `--preset` may supply channel defaults, but explicit channel flags may override only if the result remains valid and this behavior is documented;
- legacy `--level`/`--profile` combined with `--preset` should be rejected unless exact equivalence can be established without surprise;
- legacy `--dmi` combined with `--rights-policy` should be rejected on disagreement and normalized on equivalence.

Do not use “last argument wins” across unrelated semantic models unless Clap naturally guarantees order and the CLI explicitly documents it. Prefer deterministic conflict errors.

### 1.4 Typed configuration errors

Configuration failures should reach `classify_error()` as `Error::Config` or a CLI configuration error mapped to exit code 2.

Avoid calling `std::process::exit` from deep helper functions. Return errors to one top-level exit path where practical.

### Phase 1 acceptance criteria

- one builder creates every protection request;
- dry-run and execution share the same request;
- no branch discards DMI/policy overrides;
- conflicts are typed and deterministic;
- equivalent legacy/new inputs produce equivalent requests;
- focused unit tests cover the interaction table.

Suggested commit:

```text
cli: normalize all protection options into one request
```

---

## Phase 2: Collapse single and batch execution paths

### 2.1 One per-file processor

Use one function that accepts:

```text
input path
resolved output path
ProtectionRequest
output/report mode
```

It should call the request-based byte API only.

Legacy CLI syntax must not call `process_image_bytes_with_warnings` directly after this phase.

### 2.2 Output path resolution

Unify output naming and collision handling.

Requirements:

- one input + explicit file output writes that file;
- batch + output directory writes unique names;
- duplicate stems from different directories do not overwrite each other;
- parallel and sequential modes use identical deterministic naming;
- input/output identity checks remain effective;
- format extension matches actual resolved output format.

The current `compute_output_path` behavior should be reviewed carefully because it returns `None` for the first stem and delegates naming elsewhere, making parallel and sequential reasoning harder.

Prefer a precomputed output plan before parallel processing:

```rust
Vec<PlannedFileOperation>
```

This avoids a shared mutex solely for naming and makes collision errors deterministic.

### 2.3 One report path

Use `process_request_bytes_with_report` as the canonical execution primitive where its overhead is acceptable. Derive warning-only output from the report rather than rerunning alternate APIs.

Human and JSON output must describe the same fields.

### 2.4 Strict mode

Strict behavior should evaluate warnings against the actual request/preset semantics, not a synthetic legacy `ProtectionContext::legal_notice()` created only for display.

If warning severity is still profile-dependent, resolve that interpretation from the canonical request/preset in one place.

### Phase 2 acceptance criteria

- legacy and new syntax invoke the same per-file processor;
- single, sequential batch, and parallel batch use the same output plan;
- output collisions are deterministic and tested;
- JSON/human paths use the same execution result;
- strict mode evaluates actual requested semantics;
- no duplicate image processing is introduced.

Suggested commit:

```text
cli: collapse protection execution onto request reports
```

---

## Phase 3: Make legacy library APIs thin adapters

Primary paths:

```text
src/lib.rs
src/types.rs
src/async_api.rs
```

### 3.1 Central legacy translation

Implement one translation from legacy context/level to `ProtectionRequest`.

It must preserve documented behavior intentionally, including:

- level defaults;
- metadata injection overrides;
- legal metadata;
- DMI value;
- MAC key;
- output/input formats;
- intensity and seed;
- JPEG options;
- tiling/redundancy options;
- resource limits;
- metadata update policy.

Where the legacy model cannot represent a canonical request cleanly, document and test the compatibility mapping. Do not keep a second pipeline merely to avoid making the mapping explicit.

### 3.2 One executor

Legacy byte entry points should call the request resolver/executor.

Avoid this topology:

```text
legacy API -> legacy pipeline
request API -> request pipeline
```

Target:

```text
legacy API -> adapter -> ProtectionRequest -> resolver -> executor
request API -------------------------------> resolver -> executor
```

### 3.3 Deprecation scope

Do not deprecate every legacy symbol automatically. Deprecate only APIs whose contract is misleading or whose replacement is clear.

`EvidenceProfile` is already deprecated; ensure it no longer changes processing independently.

### 3.4 Async and parallel wrappers

Async wrappers should wrap the canonical byte/request operation. They should not maintain separate policy logic.

Parallel library helpers should also use the canonical operation per item.

### Phase 3 acceptance criteria

- one resolver determines effective policy/channels;
- legacy byte APIs produce equivalent results to constructed requests;
- async/parallel variants share semantics;
- no independent level/profile decision path remains;
- compatibility tests cover representative legacy configurations;
- documentation points new callers to request-based bytes APIs.

Suggested commit:

```text
api: route legacy processing through ProtectionRequest
```

---

## Phase 4: Correct the `DynamicImage` contract

### 4.1 State the technical invariant

`image::DynamicImage` contains decoded pixels, not the original PNG/JPEG/WebP container metadata.

Therefore, a function returning only `DynamicImage` cannot return XMP, EXIF, IPTC, PNG text chunks, JPEG COM segments, or WebP metadata chunks.

### 4.2 Choose a compatibility-safe disposition

Acceptable options:

#### Option A: deprecate and rename as pixel-only

Add an explicit API such as:

```rust
process_pixels(...)
apply_hidden_marker_to_pixels(...)
```

Deprecate `ProtectionPipeline::process` and any top-level `process_image` name that implies complete protection.

#### Option B: return a byte-bearing result

Introduce a type such as:

```rust
pub struct ProcessedImage {
    bytes: Vec<u8>,
    format: ImageOutputFormat,
    report: ExecutionReport,
}
```

This may be appropriate for a future API, but do not add it solely for abstraction. Existing request byte functions already provide complete output.

Preferred bounded approach: Option A plus strong documentation directing callers to byte APIs.

### 4.3 Behavior of retained pixel API

A pixel-only function may apply LSB changes to a `DynamicImage`. It must:

- not report metadata as injected;
- not claim complete rights protection;
- reject or document JPEG DCT semantics that require encoded bytes;
- use a name that indicates its limitations.

### 4.4 Examples and docs

Remove examples that show:

```rust
let protected = pipeline.process(&img, ...);
```

as a complete rights-metadata operation.

Replace them with byte-based examples.

### Phase 4 acceptance criteria

- no public docs imply a `DynamicImage` carries file metadata;
- complete protection examples use bytes;
- pixel-only APIs are explicit and report only pixel-channel outcomes;
- misleading APIs are deprecated with a clear replacement;
- source compatibility is preserved where reasonable;
- tests prove byte metadata is present only in byte output.

Suggested commit:

```text
api: make pixel-only processing contract explicit
```

---

## Phase 5: Unify key-source resolution

### 5.1 One key resolver

Use `resolve_key_input()` or a renamed equivalent for all CLI operations that accept the documented key forms.

Supported forms:

```text
literal hex
@path containing hex
- for stdin
STEGOEGGO_KEY environment variable when no explicit argument is provided
```

Explicit argument must take precedence over environment.

### 5.2 Verify mode

Verify mode must not directly call `hex::decode(args.key)` because that breaks `@file`, stdin, and environment behavior.

### 5.3 Validation

Define minimum key expectations separately for:

- HMAC key: reject empty; warn or reject unreasonably short keys according to documented security posture;
- Ed25519 private/public material: exact lengths and existing parsing rules;
- detached manifest payload key: same HMAC resolver where appropriate.

Do not treat an HMAC key as an Ed25519 key or vice versa.

### 5.4 Secret handling

Avoid printing key material. Keep private key file permissions behavior. Zeroize where existing types already support it; do not build a secret-management subsystem.

### Phase 5 acceptance criteria

- protect and verify accept the same documented HMAC key sources;
- explicit input precedence is tested;
- invalid hex and missing files produce configuration errors;
- stdin behavior is tested through a CLI integration helper where practical;
- keys never appear in JSON/human output;
- help text matches actual behavior.

Suggested commit:

```text
cli: use one key resolver across operations
```

---

## Phase 6: Remove production test-seed guessing

The normal CLI dependency currently enables:

```toml
features = ["test-seeds"]
```

### 6.1 Normal build

Remove `test-seeds` from the normal `stegoeggo-cli` dependency feature list.

### 6.2 Tests

Tests that require seed guessing should enable the feature in a test-specific way or pass explicit known seeds.

Prefer explicit known seeds in CLI integration tests. Production verification should not guess common development seeds implicitly.

### 6.3 User behavior

Users can continue to provide `--known-seeds` where supported. That is explicit caller intent and distinct from compiled-in guessing.

### Phase 6 acceptance criteria

- normal `cargo build --release -p stegoeggo-cli` does not enable `test-seeds`;
- tests remain deterministic with explicit seeds or test-only feature activation;
- production verification does not silently try development seeds;
- help/docs describe explicit known-seed behavior;
- no feature is removed from the library.

Suggested commit:

```text
cli: remove test seed guessing from production builds
```

---

## Phase 7: CLI integration and compatibility tests

Add focused CLI tests rather than a combinatorial matrix.

Required cases:

1. legacy syntax protects a PNG and emits canonical rights metadata;
2. equivalent request syntax produces equivalent effective policy/channels;
3. `--preset legal-notice --no-ai-training` applies the expected policy or is rejected only if intentionally conflicting;
4. explicit Allowed plus `--no-ai-training` is a config error;
5. conflicting `--dmi` and `--rights-policy` is a config error;
6. HMAC without a key is a config error;
7. verify with literal key works;
8. verify with `@file` works;
9. verify with environment key works;
10. dry-run output matches execution resolution;
11. batch duplicate stems produce unique deterministic outputs;
12. input/output same-file protection is rejected;
13. JSON and human execution use the same report facts;
14. pixel-only API does not claim metadata injection.

Use small generated fixtures. Do not add a large end-to-end framework.

### Phase 7 acceptance criteria

- the required cases pass;
- exit codes remain stable and documented;
- no test depends on branch-specific internal functions;
- batch tests work with jobs 1 and greater than 1 where parallel support is enabled;
- fixture count remains bounded.

Suggested commit:

```text
tests: cover unified CLI request behavior
```

---

## Phase 8: Documentation and closure

Update as applicable:

```text
README.md
src/lib.rs
stegoeggo-cli help text
architecture/overview.md
architecture/resolve.md
architecture/verification.md
architecture/async-api.md
DEPRECATIONS.md
STABILITY.md
CHANGELOG.md
AGENTS.md
```

Required documentation:

- request-based bytes API is canonical;
- legacy APIs are adapters;
- pixel-only APIs do not preserve metadata;
- mixed option behavior is explicit;
- key-source behavior is consistent;
- test-seed guessing is test/development-only;
- no release has occurred.

### Phase 8 acceptance criteria

- all README CLI examples resolve through the canonical path;
- deprecated API replacements are clear;
- no example presents `DynamicImage` output as metadata-bearing;
- CLI help and behavior agree;
- `./scripts/check.sh` passes;
- required CI/release architecture remains unchanged;
- status ledger contains exact evidence.

Suggested commit:

```text
docs: align API and CLI with the canonical request model
```

---

## Difficult behavior examples

### Example A: equivalent syntax

These should resolve equivalently:

```bash
stegoeggo in.png --no-ai-training --hidden-marker best-effort
```

```bash
stegoeggo in.png \
  --rights-policy prohibited-ai-ml-training \
  --hidden-marker best-effort
```

The exact CLI value spelling must match Clap definitions. The test should compare resolved plan fields, not raw argument objects.

### Example B: contradictory syntax

```bash
stegoeggo in.png \
  --rights-policy allowed \
  --no-ai-training
```

Expected:

```text
configuration error
exit code 2
no output file written
```

### Example C: pixel API

Forbidden documentation claim:

```text
ProtectionPipeline::process returns a protected image containing XMP and EXIF.
```

Correct claim:

```text
Byte APIs return the encoded image with rights metadata.
Pixel-only APIs can modify pixels but cannot carry container metadata.
```

---

## Required verification commands

At minimum:

```bash
cargo test -p stegoeggo
cargo test -p stegoeggo-cli
cargo test --workspace --exclude stegoeggo-fuzz --all-features
cargo check -p stegoeggo --no-default-features
./scripts/check.sh
```

Do not add a required CLI matrix workflow.

---

## Final acceptance criteria

Plan 042 is closed only when:

- all protection CLI syntax builds one `ProtectionRequest`;
- all protection execution uses one request-based byte path;
- dry-run resolves the exact executed request;
- mixed contradictions are deterministic configuration errors;
- legacy byte APIs adapt into the canonical resolver;
- async/parallel wrappers share the same semantics;
- pixel-only APIs are explicit and cannot claim metadata injection;
- verify mode honors literal, file, stdin, and environment key sources as documented;
- normal CLI builds do not enable `test-seeds`;
- batch output planning is deterministic;
- JSON/human output share execution facts;
- documentation and deprecations are current;
- no CI/release expansion occurred;
- `plans/042-status.md` contains exact evidence.

---

## Completion definition

The plan is complete when StegoEggo has one semantic control path. Compatibility syntax may remain, but it must not create compatibility behavior that bypasses or contradicts the canonical request.