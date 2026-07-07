# Plan 008: CI and Release Gates

## Goal

Make release validation visible, reproducible, and trustworthy. Previous checks have been reported in commit messages, but the repository still needs a direct audit of GitHub Actions visibility, workflow triggers, MSRV enforcement, package readiness, cargo-deny policy, and benchmark placement.

This phase should produce a clear answer to: can a maintainer look at GitHub and know the current `main` is release-clean?

## Scope

In scope:

- GitHub Actions workflow triggers and branch coverage.
- Combined status/check visibility for `main` commits.
- Local command parity with CI.
- MSRV verification.
- `cargo package` readiness.
- `cargo deny` reliability.
- Benchmark workflow policy.
- Release gate documentation.

Out of scope:

- Redesigning the legal-notice APIs.
- Adding new metadata features.
- Replacing the CI provider.

## Files to Inspect

- `.github/workflows/ci.yml`
- `.github/workflows/*.yml`
- `Cargo.toml`
- `Cargo.lock`
- `deny.toml`
- `README.md`
- `AGENTS.md`
- `CHANGELOG.md`
- `benches/bench.rs`
- Any release or packaging docs if present

## Tasks

### 1. Verify GitHub Actions Visibility

Check whether the latest `main` commit has a visible GitHub Actions run. If the status API returns empty but the GitHub UI shows a run, document the discrepancy in `AGENTS.md` or release notes only if it is repeatable and relevant.

If no run exists, inspect workflow triggers. The main CI workflow should generally include:

```yaml
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
```

If workflows are intentionally manual-only, document that. Otherwise, fix the trigger.

### 2. Audit CI Jobs

The CI should cover at least:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`
- `cargo test --doc`
- MSRV check with Rust 1.87 if the crate claims `rust-version = "1.87"`
- `cargo deny check licenses`
- `cargo deny check advisories`
- `cargo package --workspace` or equivalent package dry-run

If any of these are missing, either add them or explicitly document why not.

### 3. Check MSRV Consistency

Confirm `Cargo.toml` uses `rust-version = "1.87"` and that CI actually checks it.

Run locally if possible:

```bash
rustup run 1.87 cargo check --all-features
```

If this fails because dependencies have advanced past MSRV, either update MSRV or pin/adjust dependencies. Do not leave a false MSRV badge or claim.

### 4. Package Readiness

Run:

```bash
cargo package --workspace --allow-dirty
```

Resolve:

- Missing README/license/docs included in package.
- Path-only dependency package issues.
- Excess large generated files.
- Incorrect crate metadata.
- Invalid crates.io keyword count or keyword length.

Pay special attention to crate keywords. Crates.io allows a limited number of keywords; ensure the legal-notice keyword expansion did not exceed limits.

### 5. Cargo Deny Policy

Run:

```bash
cargo deny check licenses
cargo deny check advisories
```

Inspect `deny.toml`. If it is still largely template-like, tighten it enough for release:

- Explicit allowed licenses.
- Advisory behavior defined.
- Bans/duplicate policy set intentionally.
- Unused generated comments removed if they obscure actual policy.

Do not over-tighten to the point that ordinary patch releases become high-friction.

### 6. Benchmark Workflow Policy

Inspect whether Criterion benches run on every push to `main`.

Recommended policy:

- Normal CI: compile benches or run a short smoke benchmark only.
- Full Criterion benches: manual `workflow_dispatch`, scheduled, or release-only.

If full benches are currently running on every push, move them to manual/scheduled unless there is a strong reason to keep them.

### 7. Release Gate Documentation

Add a short release validation section to `AGENTS.md` or a release checklist doc:

```text
Release gate:
- fmt clean
- clippy clean
- all tests pass
- doc tests pass
- MSRV check passes
- cargo package dry-run passes
- cargo deny passes
- external metadata conformance script passes or skip behavior documented
```

## Required Commands

Run:

```bash
cargo fmt --check
cargo test --all-features
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
cargo package --workspace --allow-dirty
cargo deny check licenses
cargo deny check advisories
```

Optional but recommended:

```bash
rustup run 1.87 cargo check --all-features
cargo bench --no-run
```

## Acceptance Criteria

- Latest `main` commit produces a visible GitHub Actions run, or workflow limitations are corrected.
- CI workflow includes fmt, clippy, tests, doc tests, MSRV, deny, and package dry-run or equivalent release checks.
- MSRV claim is verified or updated.
- Package dry-run passes.
- Cargo deny checks pass.
- Benchmark policy is deliberate and documented.
- Release gate checklist is documented.

## Risk Notes

Do not assume local green tests equal release readiness. GitHub Actions visibility matters for public release trust.

Do not add heavyweight external metadata tools to the default CI path unless runtime and installation reliability are acceptable. External conformance can be manual or separate from core CI.

Avoid CI that is too slow for normal development. Use separate manual workflows for expensive checks.
