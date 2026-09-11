# Plan 101: CLI Installation, Documentation, and Release Closeout

## Goal

Complete the user-facing migration after Plans 098-100 by making the simplified command model, binary-first installation path, release artifact policy, and self-update behavior consistent across README, detailed docs, release instructions, support policy, architecture notes, and regression checks.

This plan is intentionally documentation-heavy but includes executable validation because installation/update documentation is part of the product contract.

## Scope

In scope:

- Rewrite installation guidance around prebuilt CLI binaries.
- Rewrite CLI examples around `protect`, `inspect`, and `verify`.
- Document legacy syntax as compatibility-only.
- Document `version` and `update`.
- Align `RELEASING.md`, `SUPPORT.md`, `STABILITY.md`, `DEPRECATIONS.md`, `SECURITY.md`, `AGENTS.md`, and `architecture/cli.md`.
- Add documentation/link/command smoke coverage where practical.
- Run an end-to-end release/install/update rehearsal.

Out of scope:

- Further CLI redesign after Plan 098.
- New release targets beyond Plan 099.
- New updater behavior beyond Plan 100.
- Package-manager integrations.

## Workstream A: README Simplification

The README should optimize for someone who wants to use the CLI directly.

Recommended top-level flow:

1. One-sentence product description.
2. Preferred install command.
3. Three-command quickstart: protect, inspect, verify.
4. Short explanation of policy versus evidence preset.
5. Link to detailed CLI/install docs.
6. Rust library section after CLI usage.

Preferred installation example:

```bash
curl -fsSL https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh | bash
```

Keep Cargo install as an alternative, not the first path:

```bash
cargo install stegoeggo-cli --locked
```

Avoid making users read feature-flag/release internals before they can protect an image.

## Workstream B: Dedicated Installation Guide

Create `docs/installation.md` modeled on the useful parts of Gregg's installation guide.

It must cover:

- supported prebuilt targets;
- exact one-command Unix install;
- Windows install if supported;
- user-local versus root/system destinations;
- PATH behavior;
- pinned version install;
- direct binary download and checksum verification;
- Cargo fallback/source-only targets;
- unsigned macOS/Windows considerations if applicable;
- `stegoeggo version` verification;
- `stegoeggo update` behavior;
- permission behavior for system installs;
- uninstall guidance (manual binary removal is sufficient unless a dedicated uninstall command is added later).

Do not claim code signing/notarization if the artifacts are only checksummed.

## Workstream C: CLI Usage Guide Rewrite

Rewrite `docs/cli-usage.md` around commands rather than the current flat-root flag inventory.

Recommended sections:

```text
Protecting images
Inspecting images
Verifying integrity/authentication
Rights policy
Evidence presets and advanced channel controls
Rights metadata fields
Batch processing
Output/format controls
Dry run and JSON
Version and updates
Signing/manifests (when compiled)
Legacy compatibility syntax
Exit codes
```

Canonical examples should use:

```bash
stegoeggo protect ...
stegoeggo inspect ...
stegoeggo verify ...
```

Move `--level`, `--profile`, `--dmi`, `--no-ai-training`, `--no-genai-training`, `--tdm-reserved`, `--metadata`, and `--legal-claims` into a clearly labeled compatibility section. Do not teach them in quickstart examples.

Clearly distinguish:

- `inspect`: reporting, generally success if the file can be inspected;
- `verify`: assertion-oriented, non-zero on integrity/authentication failure;
- legacy `--verify`: retained 0.x compatibility behavior if it still always exits 0.

## Workstream D: Release Policy Rewrite

Update `RELEASING.md` so two independent release concerns are explicit:

### Crates.io publication

Remain manual and maintainer-driven:

```text
stegoeggo-stego -> stegoeggo -> stegoeggo-cli
```

No Actions workflow receives crates.io publication credentials.

### GitHub binary artifacts

Document the Plan 099 process:

- exact source tag/version requirements;
- release workflow dispatch or tag trigger chosen during implementation;
- binary feature set;
- supported target matrix;
- checksums;
- installer attachment;
- release asset verification;
- updater requirement that exact tagged assets exist.

Remove obsolete statements such as "GitHub releases are optional" or "do not attach a single Linux binary" once binaries are a supported distribution contract. Replace them with the new explicit artifact policy.

The release sequence must ensure version authority is coherent. If Plan 100 uses crates.io as version authority, do not expose a `latest` update until the corresponding CLI crate version and exact GitHub release assets both exist.

## Workstream E: Support and Stability Contracts

Update `SUPPORT.md` to distinguish at least three concepts:

```text
compile/test assurance target
prebuilt binary distribution target
source-only supported target
```

Do not imply every assurance target has a downloadable release asset.

Update `STABILITY.md` with:

- asset naming stability expectations;
- installer URL contract (`releases/latest/download/install.sh`);
- whether `version` output first line is stable;
- updater behavior considered stable versus best-effort;
- command compatibility during 0.x.

Update `DEPRECATIONS.md` with:

- legacy root protect syntax disposition;
- legacy `--verify` disposition;
- hidden legacy flags from Plan 098;
- v1 removal/consolidation candidates.

## Workstream F: Security Documentation

Update `SECURITY.md` to explain the distribution trust model accurately.

Required points:

- HTTPS GitHub Releases is the transport/source boundary for binary assets;
- SHA-256 sidecars detect corruption/mismatch but are not an independent signature when fetched from the same release origin;
- self-update validates checksum and exact program/version before replacement;
- updater does not elevate privileges;
- unsupported targets may build through Cargo, which changes the dependency/download trust path;
- macOS/Windows artifacts may be unsigned unless signing is explicitly implemented.

Do not describe checksums as publisher authentication.

## Workstream G: Architecture and Maintainer Guidance

Update `architecture/cli.md` with:

- command parsing topology;
- implicit-root compatibility routing;
- help grouping;
- inspect/verify semantic split;
- version/update flow;
- release target/asset naming contract;
- update trust/fallback rules.

Update `AGENTS.md` with concise operational invariants:

- canonical commands;
- legacy compatibility status;
- release artifact naming;
- one-curl installer source;
- self-update hard-failure/fallback rules;
- binary workflow does not publish crates.

Do not allow `AGENTS.md` to retain stale statements that all GitHub releases are optional/manual if the new workflow materially changes that contract.

## Workstream H: End-to-End Rehearsal

Before declaring the work complete, perform a release rehearsal using a test/prerelease version or controlled tag.

Validate at least:

1. release workflow builds all supported assets;
2. each asset has a matching checksum;
3. `install.sh` installs the correct binary on Linux x86_64;
4. test one additional architecture natively or in the actual release runner;
5. installed `stegoeggo version` matches the release;
6. installed root `--help` is concise;
7. `protect`, `inspect`, and `verify` execute successfully on a fixture;
8. `update` reports current when no newer stable exists;
9. controlled older->newer update succeeds using verified assets;
10. checksum/identity mismatch test fails without replacing the current executable.

Record closeout evidence in the plan status file rather than weakening tests after the fact.

## Workstream I: Documentation Regression Checks

Add lightweight checks where valuable:

- grep/snapshot test ensuring README's preferred installer URL matches the actual release asset name;
- tests that `version` output matches what installer/updater expect;
- help snapshots for documented commands;
- script test asserting release target names match installer/updater mappings;
- docs link check if the repository already has an appropriate mechanism.

Avoid a brittle full README golden test.

## Required Checks

```bash
./scripts/check.sh
cargo test -p stegoeggo-cli --all-features
bash -n packaging/install.sh
shellcheck packaging/install.sh   # if available
cargo run -p stegoeggo-cli -- --help
cargo run -p stegoeggo-cli -- version
cargo run -p stegoeggo-cli -- protect --help
cargo run -p stegoeggo-cli -- inspect --help
cargo run -p stegoeggo-cli -- verify --help
```

Also run the Plan 099 asset checker and Plan 100 updater integration tests.

## Acceptance Criteria

- README presents binary installation as the preferred CLI path.
- One curl command installs the CLI on supported Unix targets without requiring Rust.
- Cargo remains clearly documented as fallback/alternative.
- All primary examples use the simplified command model.
- Legacy options are documented as compatibility syntax, not recommended usage.
- `version` and `update` behavior is documented consistently with implementation.
- Release docs no longer contradict the supported binary pipeline.
- Support/security/stability docs describe the actual artifact and trust model.
- End-to-end install/update rehearsal succeeds before canonicalizing the workflow.
- No documentation claims that conflict with generated `--help`.

## Dependency / Ordering

Execute after Plans 098-100 are functionally complete. Documentation may be updated incrementally during those plans, but this is the final consistency/closeout pass.
