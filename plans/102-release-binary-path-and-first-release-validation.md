# Plan 102: Release Binary Path Hardening and First-Release Validation

## Goal

Close the remaining risk in the new CLI binary-distribution path before the first 0.4.x binary release is treated as production-ready.

The CLI command cleanup, installers, binary workflow, self-update command, and documentation have landed. The remaining work is narrow:

1. make release binary staging deterministic by copying the exact Cargo output path rather than discovering it with a broad `find` fallback; and
2. exercise the complete release/install/update contract against a real GitHub Release so the documented one-curl installation path is proven rather than only fixture-tested.

This is a corrective release-readiness plan. Do not reopen the CLI information architecture, protection pipeline, stego algorithms, or library API.

## Background

The current `.github/workflows/release-binaries.yml` builds a target-specific `stegoeggo` executable and then stages it under the public versionless asset name. The staging logic first searches for `${matrix.asset}` inside `target/**/release`, even though Cargo emits `stegoeggo` on Unix and `stegoeggo.exe` on Windows. It then falls back to a broad `find ... -path '*/release/stegoeggo*'` search.

That fallback is unnecessarily nondeterministic because it can match non-executable build artifacts such as dependency files. The subsequent smoke step should prevent a bad artifact from being published, but the workflow should not rely on search ordering or downstream failure to identify the intended executable.

Separately, the stable installer URL now documented throughout the repository is:

```text
https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh
```

At the time this plan was written, the latest published GitHub Release predates the binary pipeline and has no installer/binary assets. The source implementation is therefore landed, but the public binary distribution path has not yet been proven by a real release.

## Scope

In scope:

- deterministic release binary staging for all supported targets;
- release workflow regression coverage for asset naming/path selection;
- real GitHub Release validation for the complete five-target asset contract;
- real Unix bootstrap-install validation from the documented `releases/latest` URL;
- Windows installer validation if a Windows runner/operator environment is available;
- `stegoeggo version` validation on installed binaries;
- `stegoeggo update` validation from an older real release to a newer real release once two compatible binary-enabled releases exist;
- documentation corrections only where the real release rehearsal disproves current assumptions;
- closeout evidence/status notes.

Out of scope:

- crates.io auto-publication;
- tag-triggered publication;
- code signing/notarization;
- changing the five-target support matrix unless a target is demonstrated to be infeasible;
- new CLI commands or protection features;
- refactoring the updater into a shared crate;
- changing the existing manual-release ownership model.

## Files to Inspect

- `.github/workflows/release-binaries.yml`
- `scripts/release-targets.txt`
- `scripts/release-binary-preflight.sh`
- `scripts/release-check-assets.sh`
- `scripts/test-release-installers.sh`
- `scripts/test-release-updater.sh`
- `packaging/install.sh`
- `packaging/install.ps1`
- `stegoeggo-cli/src/update.rs`
- `stegoeggo-cli/src/main.rs`
- `stegoeggo-cli/Cargo.toml`
- `README.md`
- `docs/installation.md`
- `docs/cli-usage.md`
- `RELEASING.md`
- `SUPPORT.md`
- `SECURITY.md`
- `AGENTS.md`

## Workstream A: Deterministic Cargo Output Paths

Replace broad binary discovery in `.github/workflows/release-binaries.yml` with an explicit source path derived from the Rust target and platform.

For release builds using `--target <triple>` or cargo-zigbuild `<triple>.2.17`, stage from:

```text
Unix:    target/<triple>/release/stegoeggo
Windows: target/<triple>/release/stegoeggo.exe
```

The destination remains the public versionless asset contract:

```text
stegoeggo-x86_64-unknown-linux-gnu
stegoeggo-aarch64-unknown-linux-gnu
stegoeggo-x86_64-apple-darwin
stegoeggo-aarch64-apple-darwin
stegoeggo-x86_64-pc-windows-msvc.exe
```

### Requirements

1. Remove the `find target ... stegoeggo*` fallback entirely.
2. Fail immediately if the exact expected Cargo output file does not exist.
3. Copy only that exact file into `release-assets/<public-asset-name>`.
4. Preserve executable permissions on Unix.
5. Keep the current post-copy `version`, `--help`, `protect`, and `inspect` smoke checks.
6. Do not infer executable identity from file names alone; the staged candidate must still pass the runtime version smoke.

### Suggested workflow shape

Use matrix metadata for the Cargo binary suffix rather than filesystem searching, for example conceptually:

```text
matrix.target
matrix.asset
matrix.binary_name = stegoeggo | stegoeggo.exe
```

Then:

```text
source=target/${target}/release/${binary_name}
test -f "$source"
cp "$source" release-assets/${asset}
```

For cargo-zigbuild, confirm that specifying `<target>.2.17` still writes under `target/<target>/release/`; retain that contract only after testing it explicitly.

## Workstream B: Asset Contract Regression Checks

Strengthen release validation so a future workflow edit cannot silently drift from the installer/updater target contract.

### Tasks

1. Ensure `scripts/release-targets.txt` remains the single documented target/asset inventory, or add a check proving the workflow matrix matches it exactly.
2. Extend `scripts/release-check-assets.sh` if necessary to require:
   - all five executable assets;
   - one `.sha256` sidecar per executable;
   - `install.sh`;
   - `install.ps1`;
   - no duplicate/misnamed executable variants.
3. Verify each sidecar names or hashes the intended public asset correctly.
4. Verify every staged executable reports exactly:

```text
stegoeggo X.Y.Z
```

5. Add a regression test or lightweight script assertion that the workflow's source binary name is exactly `stegoeggo` / `stegoeggo.exe`, not the public release asset name.

Do not add a second independent target list unless unavoidable. Prefer deriving/checking against `scripts/release-targets.txt` to prevent contract drift.

## Workstream C: First Real Binary Release Rehearsal

The first 0.4.x release using this pipeline is part of the acceptance criteria, not merely an operational follow-up.

### Preconditions

- crates for the selected version have been published manually in the existing carrier → library → CLI sequence;
- `stegoeggo-cli` on crates.io reports the same version as the release tag;
- an exact `vX.Y.Z` tag points to the published source commit;
- a GitHub Release exists for that exact tag;
- the release is not a prerelease if it is intended to become `releases/latest` and updater-visible.

### Procedure

1. Run the local binary release preflight against the tag.
2. Dispatch `.github/workflows/release-binaries.yml` with the exact tag.
3. Require every matrix build to succeed.
4. Require the attach job to succeed only after the complete asset validator passes.
5. Inspect the GitHub Release and confirm exactly the intended binary assets, sidecars, and installers are attached.
6. Download at least one Unix binary directly and validate `version`, `--help`, `protect`, `inspect`, and `verify` behavior.
7. On available native environments, validate the corresponding macOS/Windows artifacts rather than assuming cross-build success is sufficient.
8. Record the workflow run URL, release tag, tested hosts, and outcomes in the Plan 102 status/closeout file.

A partially attached release is not a successful binary CLI release. If one matrix job fails, do not describe the release as installer-ready until the full contract is restored.

## Workstream D: Prove the Preferred One-Curl Installer

After the binary-enabled release is published and is the current stable/latest release, test the exact command shown to users rather than a local installer path:

```bash
curl -fsSL https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh | bash
```

### Unix acceptance checks

Run in a clean/disposable user environment where practical.

Verify:

1. target detection selects the correct release asset;
2. the installer downloads the corresponding `.sha256` sidecar;
3. checksum verification happens before candidate execution/installation;
4. candidate `version` validation succeeds;
5. non-root installation lands in the documented user-local directory;
6. PATH warning behavior is correct when that directory is absent from `PATH`;
7. no shell startup file is modified automatically;
8. no `sudo` is invoked internally;
9. the installed `stegoeggo version` reports the release version;
10. `stegoeggo --help`, `protect`, `inspect`, and `verify` work after installation.

Also test a pinned installer URL for the same release:

```bash
curl -fsSL https://github.com/eggstack/stegoeggo/releases/download/vX.Y.Z/install.sh | bash -s -- --version X.Y.Z
```

If the installer syntax differs from this example, use the syntax currently documented and fix any discrepancy discovered during rehearsal.

### Windows

When a Windows environment is available, exercise the actual `install.ps1` release asset rather than only repository-local fixture tests. Validate checksum, version, destination, PATH guidance, and absence of implicit elevation.

## Workstream E: Real Self-Update Validation

A full self-update validation requires two binary-enabled stable releases. Do not fabricate success using only fixture servers once real releases are available.

When releases `A < B` both satisfy the binary asset contract:

1. install release A using its pinned installer or exact release asset;
2. run `stegoeggo version` and record A;
3. run `stegoeggo update`;
4. verify it resolves B from crates.io as the latest non-yanked stable CLI version;
5. verify it obtains the exact B GitHub asset and sidecar for the host target;
6. verify candidate identity/version before replacement;
7. verify the resulting executable reports B;
8. rerun `stegoeggo update` and confirm the already-current path is clean and non-destructive.

Also validate at least these failure modes using the existing fixture/rehearsal infrastructure if they cannot be induced safely against real releases:

- checksum mismatch leaves A untouched;
- wrong candidate identity leaves A untouched;
- missing sidecar is a hard failure;
- generic network failure does not trigger Cargo fallback;
- exact executable asset HTTP 404 may trigger Cargo fallback;
- unsupported target may trigger Cargo fallback;
- unwritable destination fails before downloading the update;
- updater never invokes `sudo`.

If only one binary-enabled release exists when Plan 102 is otherwise ready to close, mark the real A→B self-update rehearsal as explicitly pending release N+1 rather than claiming it was completed. All other Plan 102 acceptance criteria should still be completed.

## Workstream F: Documentation Reconciliation

After the real release rehearsal, review the current user-facing contract against observed behavior.

Update only where necessary:

- `README.md`
- `docs/installation.md`
- `docs/cli-usage.md`
- `RELEASING.md`
- `SUPPORT.md`
- `SECURITY.md`
- `AGENTS.md`

### Required consistency

- The one-curl command shown in README must actually work against `releases/latest` once a binary release is advertised.
- Supported binary target names must match attached assets exactly.
- Cargo fallback conditions must match implementation exactly.
- Unsigned macOS/Windows behavior must be described accurately and without implying publisher authentication from SHA-256 sidecars.
- Manual crates.io publication policy must remain unchanged.
- The binary workflow must be described as attaching assets to an existing release, not creating or publishing crates automatically.

Run `scripts/check-docs-contract.sh` after any documentation change.

## Validation Commands

At minimum before merge:

```bash
./scripts/check.sh
./scripts/release-binary-preflight.sh --tag=vX.Y.Z
./scripts/test-release-installers.sh
./scripts/test-release-updater.sh
```

For a staged asset directory:

```bash
./scripts/release-check-assets.sh --dir=<asset-directory> --version=X.Y.Z
```

Also run or inspect the manual GitHub Actions binary workflow against the real release tag as described above.

## Acceptance Criteria

Plan 102 is complete when:

- `.github/workflows/release-binaries.yml` stages executables from exact deterministic Cargo output paths and contains no broad `stegoeggo*` filesystem search fallback;
- source binary path and public asset naming are clearly separated;
- the workflow matrix and `scripts/release-targets.txt` are proven consistent;
- asset validation requires the complete five-binary + five-checksum + two-installer contract;
- all normal repository checks pass;
- at least one real 0.4.x GitHub Release successfully receives the full binary distribution matrix;
- the exact README one-curl installer command succeeds against the real `releases/latest` endpoint on a supported Unix target;
- the resulting installed binary passes `version`, help, protect, inspect, and verify smoke tests;
- Windows installer behavior is validated on Windows when an environment is available, or explicitly documented as pending native validation rather than assumed;
- updater fixture/rehearsal tests still pass;
- real release-to-release self-update is validated once two binary-enabled stable releases exist, or is explicitly left as the only pending N+1 closeout item;
- documentation reflects observed release behavior rather than intended behavior;
- a `plans/102-release-binary-path-and-first-release-validation-status.md` closeout file records the exact release tag, workflow run, tested hosts, commands, and any deferred N+1 updater validation.

## Failure Policy

Do not paper over release failures by weakening validation.

- If an expected binary path is missing, fail the build rather than searching broadly.
- If a checksum or candidate identity check fails, fail the release/install/update path; do not fall back to Cargo.
- If one supported target fails to build, the release is incomplete until corrected; do not silently drop the target from docs.
- If a platform proves genuinely unsupportable, handle that as a separate support-contract decision with explicit documentation and plan/status evidence.
- Do not manually upload ad hoc differently named binaries to make the installer appear functional.

## Handoff Notes

This plan is intentionally narrow and should follow Plans 098–101 as a release-hardening closeout. The implementation should reuse the existing target table, installer tests, updater tests, and release validation scripts rather than introducing a parallel release framework.

The primary corrective code change is small: make the build artifact source path explicit. Most of the remaining work is proving that the already-landed distribution contract survives a real release end to end.