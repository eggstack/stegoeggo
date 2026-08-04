# Plan 047: CLI Default Policy and Equivalence Corrective Pass

Status: Ready for implementation

Baseline: `main` after Plan 046 planning commit `333c095bc27aa91e65e913c98121b6ea013dbfca`

Depends on:

- `plans/045-corrective-correctness-closure-roadmap.md`

Corrects incomplete criteria from:

- `plans/042-api-cli-contract-consolidation.md`
- `plans/044-cross-format-correctness-closure.md`

Must complete before:

- `plans/050-post-corrective-evidence-and-documentation-closure.md`

---

## Purpose

Restore the documented default policy for ordinary CLI use and prove that legacy syntax and canonical request syntax resolve to the same request rather than merely both producing a file.

The CLI now routes protection through `ProtectionRequest`, which is the correct architecture. The remaining defect is in legacy normalization: the default DMI fallback is nested inside `args.dmi.as_ref().and_then(...)`. When `--dmi` is omitted, the normal `Standard` invocation can resolve to `RightsPolicy::Unspecified` instead of the historically documented `ProhibitedAiMlTraining` policy.

Existing equivalence tests only prove that both output files are nonempty. They do not compare resolved policy, channels, metadata, reports, or raw rights signals.

This plan corrects normalization without restoring a second processing path.

---

## Governing decisions

1. `ProtectionRequest` remains the only protection intent model used by CLI execution.
2. Legacy arguments are compatibility syntax translated into the canonical request model.
3. The normal default CLI invocation retains its documented legacy behavior unless the user explicitly selects canonical options that change it.
4. `--dmi auto` and omitted `--dmi` must have the same policy result for the same legacy level/profile inputs.
5. Explicit `--dmi unspecified` remains distinct from omitted/auto behavior.
6. Conflicting expressions remain configuration errors with exit code 2.
7. Dry-run and execution must use the same constructed request.
8. Single-file, sequential batch, and parallel batch must not reconstruct policy separately.
9. Do not reintroduce `test-seeds` into the production CLI.
10. Do not redesign output naming, command layout, or detached-manifest commands unless a direct regression is found while satisfying this plan.

---

## Phase 0: Create the status ledger

Create `plans/047-status.md` before source edits.

Initialize it with:

```text
Plan baseline SHA: 333c095bc27aa91e65e913c98121b6ea013dbfca
Disposition: OPEN
Default legacy policy: OPEN
Omitted versus auto equivalence: OPEN
Explicit unspecified behavior: OPEN
Legacy/request resolved equivalence: OPEN
Dry-run/execution equivalence: OPEN
Single/batch equivalence: OPEN
Conflict exit behavior: OPEN
Tests: OPEN
Documentation: OPEN
CI: OPEN
Publication hold: no publication is part of this plan
```

Add a behavior table:

```text
arguments | expected RightsPolicy | expected rights_metadata | expected hidden_marker | expected authentication | expected config result | test | status
```

Required rows:

- no policy/channel arguments;
- `--level standard`;
- `--level standard --dmi auto`;
- `--level standard --dmi unspecified`;
- `--level light`;
- `--level disabled`;
- `--no-ai-training`;
- `--no-genai-training`;
- `--tdm-reserved`;
- `--rights-policy prohibited-ai-ml-training`;
- `--preset legal-notice`;
- `--preset legal-notice-with-stego`;
- `--preset authenticated-provenance --key ...`;
- equivalent `--dmi` plus `--rights-policy`;
- conflicting `--dmi` plus `--rights-policy`;
- conflicting shorthand plus explicit policy;
- `--metadata false` plus legal fields;
- `--dry-run` variants.

---

## Phase 1: Centralize legacy policy normalization

Primary file:

```text
stegoeggo-cli/src/main.rs
```

### 1.1 Replace the nested optional fallback

Do not compute the legacy DMI value with logic whose default branch only executes when `args.dmi` is present.

Preferred shape:

```rust
fn legacy_default_dmi(level: ProtectionLevel) -> DmiValue {
    match level {
        ProtectionLevel::Disabled => DmiValue::Unspecified,
        ProtectionLevel::Light => /* preserve documented current contract */,
        ProtectionLevel::Standard => DmiValue::ProhibitedAiMlTraining,
    }
}

fn resolve_legacy_dmi(args: &Args, level: ProtectionLevel) -> DmiValue {
    match args.dmi.as_ref() {
        None | Some(DmiArg::Auto) => legacy_default_dmi(level),
        Some(explicit) => explicit.clone().into_dmi_value().unwrap_or_else(|| legacy_default_dmi(level)),
    }
}
```

The exact function signatures may differ. The critical invariant is that omitted and `Auto` use one explicit default function.

Before choosing the `Light` mapping, verify the current public documentation and pre-Plan-042 behavior. Record the chosen mapping in the status ledger. Do not infer it from a broken branch.

### 1.2 Apply shorthand overrides explicitly

The normalization sequence should be legible and deterministic:

```text
base legacy level default
explicit --dmi, if not auto
legal shorthand override, if present
conflict check against canonical explicit rights policy
conversion to RightsPolicy
```

If shorthand and explicit legacy `--dmi` disagree, define and test either conflict or documented shorthand precedence. Prefer conflict when both are explicit user claims.

Do not rely on incidental `else if` order without a documented rule.

### 1.3 Preserve disabled semantics

For `--level disabled` with no legal fields or explicit policy:

- no rights metadata;
- no hidden marker;
- no authentication;
- byte-preserving behavior remains expected where already supported.

If the user supplies explicit legal metadata or a rights policy with disabled level, use the existing documented validation/translation behavior. Do not silently discard explicit rights claims.

### Phase 1 acceptance criteria

- omitted `--dmi` and `--dmi auto` resolve identically;
- normal `Standard` invocation restores the documented policy;
- explicit `--dmi unspecified` remains explicit;
- level defaults are implemented in one function;
- shorthand behavior is deterministic;
- disabled behavior remains correct;
- no alternate executor is introduced.

Suggested commit:

```text
cli: restore canonical request defaults for legacy syntax
```

---

## Phase 2: Expose a testable resolved-request view

The current CLI tests cannot reliably compare requests if all normalization remains trapped behind process execution and human formatting.

Use the smallest testability improvement.

Acceptable approaches:

### Option A: Unit-test the private builder in `main.rs`

Add a `#[cfg(test)]` module that constructs parsed `Args` and calls `build_protection_request()` directly.

### Option B: Move argument normalization into a small CLI library module

For example:

```text
stegoeggo-cli/src/config.rs
```

with crate-private types/functions used by `main.rs` and integration tests.

Do not create a generalized command framework. Prefer Option A unless integration tests need stable access to exact request fields.

### Required comparison fields

Equivalent syntax tests must compare at least:

```text
rights policy
effective channels
seed
intensity
output format
JPEG quality/progressive setting
legal metadata fields
MAC-key presence and bytes where safe in tests
metadata update policy
resource limits if CLI-configurable
```

If `ProtectionRequest` lacks public getters needed for tests, add only narrowly justified read-only accessors or compare the resolved plan/dry-run JSON. Do not expose secret bytes in production human output.

### Phase 2 acceptance criteria

- tests can inspect the actual normalized request or resolved plan;
- no test relies only on output-file nonemptiness for semantic equivalence;
- secret material is not printed in normal output;
- production execution still calls the same builder.

Suggested commit:

```text
tests: make CLI request normalization directly observable
```

---

## Phase 3: Strengthen exact CLI equivalence tests

Primary file:

```text
stegoeggo-cli/tests/cli.rs
```

and optional unit-test module from Phase 2.

### 3.1 Default regression test

Run normal legacy syntax with fixed seed:

```bash
stegoeggo input.png --output out --seed 42
```

Assert through dry-run JSON, report JSON, or raw output verification:

```text
RightsPolicy::ProhibitedAiMlTraining
rights metadata enabled
hidden marker enabled according to Standard legacy behavior
canonical PLUS URI present
```

This test must fail under the audited broken implementation.

### 3.2 Omitted versus auto

Compare:

```bash
stegoeggo input.png --seed 42
stegoeggo input.png --seed 42 --dmi auto
```

Assert equivalent normalized requests and semantic reports.

Do not require byte equality if timestamps or nondeterministic output fields differ. Use fixed seed and deterministic timestamps where available, then compare resolved plans and extracted semantics.

### 3.3 Explicit unspecified

Compare the default invocation against:

```bash
stegoeggo input.png --seed 42 --dmi unspecified
```

Assert that explicit unspecified does not silently inherit the default prohibition policy.

After Plan 046, raw output for explicit unspecified must contain no `plus:DataMining` property.

### 3.4 Legacy versus canonical policy syntax

Required equivalent pairs:

```text
--dmi prohibited-ai
--rights-policy prohibited-ai-ml-training

--no-ai-training
--rights-policy prohibited-ai-ml-training plus equivalent constraints metadata

--dmi allowed
--rights-policy allowed
```

Compare normalized policy and channels. Compare legal metadata only when both invocations express equivalent legal fields.

### 3.5 Preset versus legacy channel syntax

Add representative pairs only where semantics are truly equivalent. Do not force equivalence between syntax families when presets intentionally differ.

For each claimed equivalent pair, compare:

```text
policy
rights_metadata
hidden_marker
authentication
report outcome
```

### 3.6 Conflict behavior

Assert exact exit code 2 and no output creation for:

- explicit Allowed plus `--no-ai-training`;
- conflicting `--dmi` and `--rights-policy`;
- HMAC with no key;
- HMAC with hidden marker disabled;
- incompatible preset plus nondefault legacy level/profile;
- metadata disabled plus legal fields.

Do not only assert `!status.success()`.

### Phase 3 acceptance criteria

- default policy regression has a focused test;
- omitted/auto equivalence is exact;
- explicit unspecified is distinct;
- claimed legacy/request equivalence compares semantic fields;
- conflict tests assert exit code and no output;
- tests use fixed seed and controlled inputs;
- test count is bounded to meaningful combinations.

Suggested commit:

```text
tests: enforce CLI policy and channel equivalence
```

---

## Phase 4: Dry-run, JSON, human, and batch consistency

### 4.1 One constructed request

Trace the CLI flow and confirm that `build_protection_request()` is called once per invocation, not independently for dry-run, display, and execution.

If display-specific code still reconstructs `EvidenceProfile` or warning semantics from raw arguments, centralize that interpretation on the canonical request/preset without changing the processing model.

### 4.2 Dry-run parity

For a fixed invocation:

- capture dry-run resolved policy/channels;
- execute the same invocation without `--dry-run`;
- verify the report/output reflects the same policy/channels.

Do not compare only a string such as `Metadata-only: true`.

### 4.3 JSON and human output

Both output modes must derive from the same execution report. Add one test that checks shared material facts, such as:

```text
effective policy
metadata injected
stego attempted
stego succeeded or downgrade
output format
warning count/severity
```

Human text need not be machine parsed exhaustively. Check representative values.

### 4.4 Batch consistency

Run the same request in:

```text
single-file mode
sequential batch jobs=1
parallel batch jobs>1
```

Verify every output extracts the same requested policy and channel outcome.

Do not add timing assertions or performance gates.

### Phase 4 acceptance criteria

- request construction is single-source;
- dry-run matches execution semantics;
- JSON and human modes use one report;
- batch modes do not rebuild policy differently;
- output naming remains deterministic;
- no new concurrency abstraction is introduced.

Suggested commit:

```text
cli: align dry-run reports and batch execution semantics
```

---

## Phase 5: Documentation correction

Inspect:

```text
README.md
architecture/cli.md
AGENTS.md
STABILITY.md
DEPRECATIONS.md
CHANGELOG.md
```

Document:

- normal default policy;
- omitted `--dmi` and `--dmi auto` equivalence;
- explicit `--dmi unspecified` distinction;
- conflict rules;
- canonical request model;
- key source behavior;
- no `test-seeds` in production CLI.

Avoid documenting internal helper names as stable API.

### Phase 5 acceptance criteria

- defaults match code and tests;
- legacy syntax is clearly compatibility syntax;
- canonical syntax is recommended without breaking old examples;
- conflicts and exit code 2 are documented;
- no claim implies two processing engines.

Suggested commit:

```text
docs: correct CLI defaults and normalization contract
```

---

## Required verification

Run at minimum:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p stegoeggo-cli --test cli
cargo test --workspace --all-features
cargo check -p stegoeggo --no-default-features
./scripts/check.sh
```

Do not add a CLI-specific CI job; existing workspace CI is sufficient.

---

## Definition of done

Plan 047 is complete only when:

1. Normal legacy `Standard` invocation resolves to the documented default rights policy.
2. Omitted `--dmi` equals `--dmi auto`.
3. Explicit `--dmi unspecified` remains distinct and emits no DMI after Plan 046.
4. One helper owns legacy level-to-policy defaults.
5. Shorthand and explicit-policy conflicts are deterministic.
6. Legacy and canonical syntax tests compare actual resolved semantics.
7. Dry-run and execution use the same request.
8. JSON and human output derive from the same report.
9. Single and batch modes preserve the same request semantics.
10. Conflict tests assert exit code 2 and no output.
11. Production CLI remains free of `test-seeds`.
12. `plans/047-status.md` records exact behavior choices, commits, commands, and results.
13. `./scripts/check.sh` passes.
14. No release or publication action occurs.