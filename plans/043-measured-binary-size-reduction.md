# Plan 043: Measured Binary Size Reduction Without Feature Loss

Status: Ready for implementation

Baseline: `main` after Plan 042 planning commit `f2222742e022a1c22247e7d72b39eff7f30de248`

Depends on:

- `plans/038-rights-metadata-format-correctness-roadmap.md`
- `plans/039-plus-iptc-rights-metadata-correctness.md`
- `plans/040-jpeg-dct-container-correctness-and-containment.md`
- `plans/041-webp-container-xmp-exif-correctness.md`
- `plans/042-api-cli-contract-consolidation.md`

Must be completed before:

- `plans/044-cross-format-correctness-closure.md`

---

## Purpose

Reduce the installed StegoEggo CLI and normal library dependency surface through measurements, feature boundaries, and low-risk release-profile changes, without deleting capabilities or weakening format correctness.

This plan is intentionally last among implementation phases. Correctness and API consolidation determine which code is truly required by the normal artifact. Size work performed earlier would risk optimizing duplicated or incorrect behavior.

The plan does not prescribe a percentage target. It requires a reproducible baseline, evidence for each retained change, and a rule against architectural complexity for negligible savings.

---

## Governing decisions

1. Measure the stripped release artifact, not an unstripped debug build.
2. Record target triple, Rust version, linker, and enabled features.
3. Change one major variable at a time.
4. Preserve every current capability through a feature, separate binary, or equivalent implementation.
5. The default CLI must retain its documented user-visible operations unless a capability is demonstrably not exposed there.
6. Do not trade parser/container correctness for size.
7. Do not replace mature dependencies with custom code unless the custom code already exists, is simpler, and is independently verified.
8. Feature flags must correspond to coherent capabilities, not individual functions.
9. If a change saves less than approximately 2% of the stripped CLI and adds meaningful maintenance complexity, revert it unless it materially simplifies the dependency graph.
10. No size diagnostics become required CI or release gates.
11. No release is performed by this plan.

---

## Required end state

- the repository records a baseline and final stripped CLI size;
- `cargo tree -e features` and symbol/dependency evidence identify dominant contributors;
- release-profile settings are evaluated and only beneficial settings retained;
- optional library capabilities do not force unrelated dependencies into minimal consumers;
- the normal CLI does not enable test-only seed guessing;
- conformance tooling does not enlarge normal library/CLI artifacts unnecessarily;
- ISCC/content-identifier support remains available but may be feature-gated if it materially affects size;
- parallel processing remains available, either through Rayon or a justified smaller implementation/feature boundary;
- duplicate JPEG encoder/decoder dependencies are retained only when behavior requires both;
- the final artifact passes the same product tests as the baseline;
- the required CI and manual release model remain unchanged.

---

## Non-goals

Do not use this plan to:

- remove image formats;
- remove signing, detached manifests, async, parallel, ISCC, conformance, or other capabilities entirely;
- reduce resource limits or parser validation;
- add unsafe code;
- write a custom allocator;
- introduce UPX or runtime executable compression as the primary solution;
- add build scripts that download tools;
- add a custom xtask/Make/Just framework;
- chase a fixed marketing size target;
- make platform-specific linker assumptions without documenting them;
- add required binary-size CI;
- publish a release.

---

## Phase 0: Create the size ledger and freeze behavior

Create `plans/043-status.md` before manifest/profile edits.

Initialize it with:

```text
Plan baseline SHA: f2222742e022a1c22247e7d72b39eff7f30de248
Disposition: OPEN
Baseline stripped CLI size: UNMEASURED
Release profile: OPEN
Feature graph: OPEN
ISCC gating: OPEN
Conformance/TOML gating: OPEN
Parallel/Rayon decision: OPEN
JPEG dependency overlap: OPEN
Clap feature review: OPEN
Final stripped CLI size: UNMEASURED
Publication hold: no publication is part of this plan
```

Add these tables.

### Table A: measurement environment

```text
field | value
```

Required fields:

- commit SHA;
- target triple;
- host OS/architecture;
- `rustc -Vv`;
- Cargo version;
- linker;
- build command;
- enabled features;
- strip method;
- file-size command;
- binary path.

### Table B: experiment log

```text
experiment | change | build command | stripped bytes | delta bytes | delta percent | behavior checks | complexity | keep/revert | commit
```

### Table C: dependency ownership

```text
dependency | capability | normal CLI required | minimal library required | optional feature candidate | measured evidence | disposition
```

Required rows:

- `image`;
- `jpeg-encoder`;
- `serde`/`serde_json`;
- `toml`;
- `iscc-lib`;
- `rayon`;
- `tokio`;
- `ed25519-dalek`;
- `clap`;
- `tempfile`;
- hashing/HMAC dependencies;
- `unicode-normalization`;
- `thiserror`.

### Phase 0 acceptance criteria

- the baseline is built after Plans 039-042 implementation, not from the planning-only SHA;
- commands and environment are reproducible;
- a behavior smoke set is fixed before experiments;
- no dependency is classified optional solely from its name;
- uncommitted local build noise is excluded.

Suggested commit:

```text
plans: establish binary size measurement ledger
```

---

## Phase 1: Establish reproducible baseline measurements

### 1.1 Build variants

At minimum measure:

```bash
cargo build --release -p stegoeggo-cli --bin stegoeggo
```

Record the normal default-feature artifact.

Also record diagnostic variants without treating them as target products:

```bash
cargo build --release -p stegoeggo-cli --bin stegoeggo --no-default-features
cargo build --release -p stegoeggo-cli --bin stegoeggo --features signatures
cargo build --release --bin stegoeggo-conformance
```

Adjust commands to actual feature topology after Plan 042.

### 1.2 Strip consistently

Prefer manifest-level stripping only after profile experiments. For baseline, copy the binary and apply the platform's standard strip command, recording it exactly.

Do not compare stripped and unstripped artifacts.

### 1.3 Analyze contributors

Use available tools locally:

```bash
cargo bloat --release -p stegoeggo-cli --bin stegoeggo -n 50
cargo bloat --release -p stegoeggo-cli --bin stegoeggo --crates
cargo tree -p stegoeggo-cli -e features
cargo tree -p stegoeggo-cli -d
```

If `cargo-bloat` is unavailable, record that and use linker map or dependency evidence. Do not add it to required CI.

### 1.4 Behavior smoke set

Before each retained experiment, run at least:

- protect/verify PNG metadata;
- protect/verify supported JPEG behavior;
- JPEG unsupported fallback fixture;
- WebP metadata insertion and external decode fixture;
- CLI dry-run;
- batch naming test;
- signatures/manifest tests when their feature is affected.

### Phase 1 acceptance criteria

- baseline size is recorded in bytes;
- top crate and symbol contributors are recorded;
- duplicate dependency versions are inventoried;
- behavior smoke tests pass at baseline;
- measurements are not added to CI.

Suggested commit:

```text
plans: record StegoEggo release size baseline
```

---

## Phase 2: Evaluate release-profile settings

The workspace currently lacks an explicit size-oriented release profile.

Test settings independently or in a small controlled sequence:

```toml
[profile.release]
strip = "symbols"
lto = "thin"
codegen-units = 1
panic = "abort"
```

Also test, but do not assume superiority:

```toml
opt-level = "s"
opt-level = "z"
lto = true
```

### 2.1 Compatibility considerations

- `panic = "abort"` changes panic unwinding behavior. Confirm no public FFI boundary or caller contract requires unwinding from the CLI. Consider applying it only to the CLI release profile if workspace inheritance permits a clear setup.
- full LTO can substantially increase build time. Retain it only if the size benefit is meaningful relative to thin LTO.
- one codegen unit can slow builds. This is acceptable for release builds if documented, but not if savings are negligible.
- `strip = "symbols"` affects distributed debugging. Keep debug information policy explicit.

### 2.2 Profile scope

Avoid forcing size-optimized panic/build behavior on downstream library consumers. Cargo profile settings in a library manifest do not control a downstream application's profile, but workspace release settings affect local binaries/examples. Document this clearly.

### Phase 2 acceptance criteria

- each profile experiment has a measured delta;
- retained settings pass the behavior smoke set and `./scripts/check.sh`;
- build-time cost is qualitatively recorded;
- no profile setting is retained solely because it is conventionally called size-optimized;
- final profile is small and documented.

Suggested commit:

```text
build: apply measured release size profile
```

---

## Phase 3: Feature-gate coherent optional library capabilities

### 3.1 ISCC/content identifiers

`iscc-lib` is currently unconditional while content identifiers are not required for basic metadata writing, hidden markers, or ordinary verification.

Evaluate an `iscc` feature:

```toml
[features]
iscc = ["dep:iscc-lib"]
```

Gate the ISCC module/re-exports and document the feature.

Capability preservation requirement:

- ISCC APIs remain available when enabled;
- published docs show the feature;
- tests run with all features;
- minimal library consumers do not compile `iscc-lib` when not requested.

CLI decision:

- if the CLI exposes no ISCC command, do not enable the feature in the normal CLI;
- if it does expose a user-visible operation, retain it or move it to an explicitly named feature-enabled CLI build without deleting the capability.

### 3.2 Conformance and TOML

The conformance binary and manifest parsing use TOML. Normal image protection should not require TOML unless another runtime feature uses it.

Evaluate:

```toml
conformance = ["dep:toml"]
```

Mark the conformance binary with:

```toml
required-features = ["conformance"]
```

Preserve the manual external/conformance workflow by enabling the feature explicitly.

### 3.3 Detached manifests and JSON

Do not assume `serde_json` can be removed from the normal CLI. JSON output is a normal CLI feature. Detached manifests are already feature-gated, but shared JSON use must be mapped accurately.

### 3.4 Async and signatures

These are already optional. Verify they do not leak dependencies through unconditional imports, docs, examples, or CLI features.

### 3.5 Feature topology constraints

Keep the feature list coherent and small. Avoid features such as:

```text
no-rayon
small-cli
without-iscc-but-with-json
jpeg-parser-only
```

Prefer positive capability names.

### Phase 3 acceptance criteria

- optional dependencies are expressed with `dep:` feature linkage;
- minimal library builds omit optional graphs;
- all capabilities remain available under documented features;
- docs.rs/all-features still builds;
- manual conformance workflow explicitly enables its feature;
- default CLI behavior is unchanged;
- retained gating yields a measured or clear downstream compile-surface benefit.

Suggested commit:

```text
build: gate optional ISCC and conformance capabilities
```

---

## Phase 4: Evaluate Rayon and parallelism without feature reduction

Parallel batch processing is user-visible. Do not simply remove it.

Evaluate three options.

### Option A: retain Rayon

Retain if:

- measured contribution is modest;
- implementation remains substantially simpler and safer;
- default CLI parallel batch behavior is important.

### Option B: feature-gate parallel support

Use a `parallel` feature if:

- normal minimal library consumers benefit;
- the CLI can continue enabling it by default, preserving CLI behavior;
- serial APIs remain available without Rayon.

This reduces minimal library dependency surface even if it does not reduce the normal CLI.

### Option C: replace with a small standard-library worker implementation

Only consider this if measurements show a material CLI reduction and the replacement is demonstrably simpler than keeping Rayon.

Required semantics:

- bounded worker count;
- deterministic precomputed output paths;
- no unbounded thread-per-file behavior;
- errors collected consistently;
- no custom work-stealing scheduler;
- no unsafe code.

Do not build a thread-pool framework for a small size win.

### Phase 4 acceptance criteria

- the chosen option is based on measured size and code complexity;
- `--jobs` behavior remains available;
- jobs=1 and jobs>1 produce equivalent outputs/reporting;
- no overwrite races remain;
- minimal library builds can avoid Rayon if a coherent feature boundary is retained;
- status ledger records why alternatives were rejected.

Suggested commit, only if a change is justified:

```text
build: isolate parallel processing dependency
```

---

## Phase 5: Evaluate duplicate JPEG/image codec machinery

The project uses the `image` crate with JPEG decoding support and a separate `jpeg-encoder` dependency for quality/progressive encoding.

### 5.1 Map distinct requirements

Record which dependency provides:

- JPEG decoding;
- PNG decoding/encoding;
- WebP decoding/lossless encoding;
- JPEG quality control;
- progressive JPEG encoding;
- output color conversion;
- custom DCT path inputs.

### 5.2 Test consolidation

Investigate whether the pinned `image` version's JPEG encoder can satisfy the required quality/progressive behavior. Do not assume it can.

If one dependency can replace the other without behavior loss:

- compare output correctness and fixture behavior;
- measure artifact size;
- remove only the redundant path.

If the dependencies have distinct required behavior, retain both and document that result. A completed investigation with “retain” is a valid outcome.

### 5.3 DCT containment interaction

If Plan 040 feature-gates or disables the custom DCT encoder by default, remeasure the dependency graph. Do not remove JPEG decoding needed for input validation or format conversion.

### Phase 5 acceptance criteria

- duplicate functionality is mapped accurately;
- any dependency removal preserves progressive/quality behavior or documents intentional behavior changes already approved elsewhere;
- PNG/WebP behavior is unaffected;
- size delta is measured;
- no new custom codec code is added to save dependency bytes.

Suggested commit, only if justified:

```text
build: remove redundant JPEG encoding path
```

---

## Phase 6: Review CLI-only dependencies and features

### 6.1 Clap

Inspect Clap's active feature graph.

Retain derive and required standard functionality. Evaluate disabling optional features such as color or suggestions only if:

- current CLI does not rely on them;
- help/error behavior remains acceptable;
- measured savings are nontrivial.

Do not replace Clap with a custom parser for size.

### 6.2 Tempfile

`tempfile` supports safe same-directory atomic writes.

Do not replace it with predictable temporary filenames or a fragile custom implementation merely for size.

Evaluate whether a small, correct existing std-based implementation is already present after Plan 042. Otherwise retain `tempfile` unless measurement shows it dominates and a safe cross-platform replacement can be implemented with less code and equal semantics.

### 6.3 Duplicate direct dependencies

The CLI directly depends on crates also provided through the library, including hashing/serialization/image utilities.

Remove direct dependencies only if CLI code no longer uses them after Plan 042. Cargo does not duplicate a same-version crate solely because both packages depend on it, but unnecessary direct dependencies obscure ownership and features.

### 6.4 Test-only dependencies

Ensure `base64`, duplicate `sha2`, `hex`, and other test helpers remain under `dev-dependencies` only where appropriate.

### Phase 6 acceptance criteria

- active Clap features are intentional;
- atomic-write safety is preserved;
- unused direct dependencies are removed;
- test-only dependencies do not leak into normal builds;
- every retained change has measurement evidence;
- no custom CLI parser or unsafe atomic writer is introduced.

Suggested commit:

```text
build: trim unused CLI dependency features
```

---

## Phase 7: Code-level simplification only where measured

Potential low-risk cleanups include:

- deduplicating generic monomorphized helper paths;
- reducing duplicated legacy/request code already addressed by Plan 042;
- replacing large format strings or duplicate static tables only where symbol evidence shows material contribution;
- moving specialist code behind existing coherent features.

Do not spend time on likely negligible changes such as:

- replacing zero-sized protector structs;
- removing `Arc` or `LazyLock` solely for binary size;
- hand-optimizing error messages without evidence;
- collapsing readable modules into one file;
- removing documentation from source;
- reducing validation branches.

### Phase 7 acceptance criteria

- each code-level size edit cites symbol/crate evidence;
- readability is not materially reduced;
- no correctness check is deleted;
- no public behavior changes without prior plan authorization;
- negligible experiments are reverted and recorded.

Suggested commit:

```text
refactor: remove measured release code duplication
```

---

## Phase 8: Final measurement and documentation

### 8.1 Rebuild cleanly

Use a clean target directory or `cargo clean` only when appropriate. Record the exact final commands.

Measure:

- normal stripped CLI;
- minimal library dependency build characteristics;
- signatures-enabled CLI if distributed;
- conformance binary separately.

### 8.2 Report deltas

Record:

```text
baseline bytes
final bytes
absolute delta
percent delta
retained features
feature-gated capabilities and enable commands
build-time tradeoffs
```

Do not claim cross-platform sizes based on one target.

### 8.3 Documentation

Update as applicable:

```text
Cargo.toml
stegoeggo-cli/Cargo.toml
README.md
src/lib.rs feature table
RELEASING.md
architecture/overview.md
CHANGELOG.md
STABILITY.md
AGENTS.md
```

`RELEASING.md` may document the release build command, but release remains manual and outside this plan.

### Phase 8 acceptance criteria

- final measurement is reproducible;
- all normal CLI features remain available;
- optional capability enablement is documented;
- minimal/no-default/all-feature checks pass;
- `./scripts/check.sh` passes;
- no binary-size CI gate is added;
- no release is published;
- `plans/043-status.md` is complete.

Suggested commit:

```text
docs: record measured binary size and feature boundaries
```

---

## Required verification commands

At minimum:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
cargo build --release -p stegoeggo-cli --bin stegoeggo
./scripts/check.sh
```

For every new coherent feature boundary, also run the applicable targeted combination, but do not create an exhaustive feature powerset.

Examples:

```bash
cargo check -p stegoeggo --features iscc
cargo check -p stegoeggo --features conformance
cargo build --release --bin stegoeggo-conformance --features conformance
```

Adjust to the actual final feature names.

---

## Final acceptance criteria

Plan 043 is closed only when:

- baseline and final stripped CLI sizes are recorded in bytes;
- measurement environment is recorded;
- retained release-profile settings have measured benefit;
- optional ISCC/conformance dependencies are gated if justified and capabilities remain available;
- parallel processing remains available;
- duplicate JPEG/image dependency paths are resolved or truthfully retained with evidence;
- normal CLI does not enable test-only seed guessing;
- unused CLI dependencies/features are removed where measured;
- no correctness/resource-limit behavior is weakened;
- no speculative complexity remains for negligible savings;
- minimal and all-feature builds pass;
- required CI/release architecture remains unchanged;
- no publication occurred;
- `plans/043-status.md` contains the experiment log and final disposition.

---

## Completion definition

The plan is complete when the repository can state exactly why the release artifact is its current size, what was reduced, what was retained, and how every optional capability remains available. A modest verified reduction is preferable to a dramatic but fragile rewrite.