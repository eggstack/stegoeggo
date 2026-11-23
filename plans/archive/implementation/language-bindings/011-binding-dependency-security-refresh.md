# Language Bindings Milestone 011 — Binding Dependency Security Refresh

> Archived as closed. Completion evidence: `plans/closure/language-bindings/011-status.md`; current status: `plans/subsystems/language-bindings-roadmap.md`.

Status: closed
Repository baseline: `3d2388638b0d5ed3fb01b76f9b80b2a3f25746cb` (`main`)
Source roadmap: `plans/subsystems/language-bindings-roadmap.md#12-milestone-status-table`
Long-term requirements: `plans/000-long-term-specification.md#2-canonical-api-invariants`, `plans/000-long-term-specification.md#5-release-invariants`, `plans/001-terminology-and-domain-model.md#core-request-model`
Applicable ADRs: `plans/adrs/ADR-0005-foreign-language-bindings.md`
Primary class: infrastructure

## 1. Objective

Refresh the three excluded binding Cargo lockfiles to their highest compatible dependency versions and move the Python binding from the vulnerable PyO3 0.27 line to a patched PyO3 release (RustSec fix threshold `>=0.29.0`), adapting internal wrapper code only as needed while preserving all published Python, Node, and C contracts.

## 2. Why this milestone is ready

The 2026-11-17 audit found no advisory in the C or Node binding lockfiles. The Python lockfile contains `pyo3 0.27.2`, affected by RUSTSEC-2026-0176 and RUSTSEC-2026-0177; both list PyO3 `>=0.29.0` as the fix. `bindings/python/Cargo.toml` currently specifies `pyo3 = "0.27"`, so an ordinary lockfile refresh cannot select the fix.

No binding API or ABI redesign is required. ADR-0005 and the initial M010 cross-platform qualification are closed; the existing C, Node, and Python path-filtered compatibility workflows provide consumer evidence. The binding lockfiles are independent from the root workspace lock and from maintenance-quality M006.

## 3. Current implementation evidence

- `bindings/c`, `bindings/node`, and `bindings/python` are nested Cargo workspaces excluded from the root workspace; each has its own `Cargo.lock` and its own path-filtered workflow.
- `cargo audit` scanned 55 C-lockfile packages and 80 Node-lockfile packages with no advisories; the Python lockfile scan found two PyO3 vulnerabilities among 68 packages.
- The no-write update previews reported 8 C, 16 Node, and 10 Python packages to relock at highest compatible versions. The Node preview included `napi 3.13.0 -> 3.16.2`, `napi-build 2.5.0 -> 2.6.0`, and `napi-derive 3.6.9 -> 3.7.1`.
- The Python preview could not leave PyO3 0.27 because of its manifest constraint. PyO3 0.29+ requires an explicit manifest change and compatibility check.
- The binding convention skill requires `panic = "unwind"` in both profiles, `default-features = false` on the exact local `stegoeggo` dependency, and no addition of bindings to the root workspace.
- No binding manifest or lockfile has been changed at this baseline.

## 4. Invariants that must not regress

- Preserve ADR-0005: each binding remains a leaf frontend over the canonical Rust byte API; no binding-specific dependency moves into the root library or carrier.
- Keep `stegoeggo = { version = "=0.4.2", default-features = false }` in all three bindings and do not add `bindings/` to the root workspace or `scripts/check.sh`.
- Preserve `panic = "unwind"` for C, Node, and Python in both dev and release profiles.
- Preserve the C ABI v1 symbols and numeric/error/ownership contracts; do not regenerate or change its public interface for a dependency refresh.
- Preserve Node generated declaration/runtime behavior and Python public API, abi3-Python-3.11 support, `extension-module` feature behavior, and GIL release via PyO3's supported detach API around blocking Rust work.
- Keep the documented platform support matrices and manual-only artifact publication policy unchanged.

## 5. Scope (in / explicitly out)

**In scope**

- Refresh `bindings/c/Cargo.lock` and `bindings/node/Cargo.lock` within existing manifest constraints.
- Refresh `bindings/python/Cargo.lock` and change its PyO3 requirement to a patched compatible line at or above 0.29.0.
- Make minimal PyO3 wrapper changes required by the version upgrade while preserving public names, exceptions, type behavior, GIL release, and Python 3.11 abi3.
- Run each binding's actual compatibility workflow and the separate RustSec audit for each lockfile.
- Update binding-specific version guidance if it still names PyO3 0.27 after the implementation.

**Explicitly out of scope**

- C ABI v1 signature, symbol, error-code, ownership, or stability changes.
- Python/Node public API redesign, a new language/runtime, a change to the supported Python or Node matrix, or automatic publishing.
- Unrelated major dependency migrations or changing the root workspace graph.
- Adding specialist checks to `scripts/check.sh` or changing required CI policy.

## 6. Required production changes

- Refresh each binding lockfile with Cargo; do not hand-edit package versions or checksums.
- Keep C and Node manifest constraints unless a named advisory or required compatible resolution demands a narrow change.
- Change the PyO3 dependency requirement from 0.27 to a patched 0.29-or-later compatible line, then resolve and test the Python binding with its own lockfile.
- Adapt `bindings/python/src/lib.rs` only if PyO3 API changes require it. Preserve `py.detach` semantics (or the documented equivalent in the selected PyO3 line) and ensure Python objects are accessed only with the GIL held.
- Update `architecture/bindings-python.md` and `.skills/bindings/SKILL.md` so they describe the implemented PyO3 line and verified API spelling. Do not change generated C headers or Node declarations unless a tool-driven drift check requires a matching generated artifact, and review any such diff separately.

## 7. Ordered work packages (intent, changes, acceptance evidence)

1. **Refresh the C binding lock.** Resolve compatible updates and run the C binding checks. Acceptance: C advisory scan is clean, `--locked` checks/tests pass, header and export manifest remain unchanged, and native C/C++ plus Rust parity checks pass.
2. **Refresh the Node binding lock.** Resolve compatible updates (including the available napi-rs updates where current constraints permit), run build/tests/typecheck, and verify generated-file drift. Acceptance: binding tests, TypeScript consumer contract, and Node runtime smoke matrix are green with no unexplained generated-file diff.
3. **Upgrade Python PyO3.** Change the manifest constraint to a patched line, update the lock, adapt only required internal calls, and run Rust unit tests plus the real wheel/install/pytest path. Acceptance: no PyO3 advisories remain, public Python API and error/security behavior are preserved, and Python 3.11 abi3 wheel tests pass.
4. **Re-audit all binding locks and qualify artifacts.** Run `cargo audit` for C, Node, and Python lockfiles. Run the lightweight path-filtered workflows; manually dispatch `release-python.yml` for the candidate SHA to qualify the existing five-target wheel matrix before making cross-platform wheel claims. Acceptance: all three audits are clean and the Python native wheel/smoke matrix completes successfully.
5. **Record closure.** Update the Python architecture deep-dive and bindings skill, then create `plans/closure/language-bindings/011-status.md` with exact dependency versions, workflow evidence, and any skipped manual qualification. Update the roadmap and registry from implementation evidence.

## 8. Failure, cancellation, restart, contention semantics

- If PyO3 0.29+ requires a Rust MSRV increase or cannot preserve the current Python support contract, stop and report the exact blocker; do not weaken abi3, panic containment, or the Rust 1.89 baseline without a separate maintainer decision.
- If a binding workflow fails, do not rely on `./scripts/check.sh`; it does not build any binding crate. Identify and fix the owning binding before declaring the milestone complete.
- If generated Node declarations or C header/export files change, inspect the generator output and contract diff. Never hand-edit generated files or accept an ABI change as incidental dependency churn.
- If dependency resolution or a RustSec database refresh fails, record the unavailable evidence and stop; do not add an advisory exception to make the audit green.
- The manual wheel qualification workflow creates artifacts only and must not publish to PyPI. If cancelled, rerun the same candidate SHA and record the new run rather than inferring success.

## 9. Compatibility and migration

The upgrade changes Rust implementation dependencies only. Preserve the Python and Node user-facing surfaces, C ABI v1, package version `0.4.2`, and the exact local library dependency. Verify PyO3 0.29+ against Rust 1.89 and the existing CPython 3.11 abi3 wheel contract. If PyO3 requires an externally visible support change, stop and request a separate decision instead of bundling it here.

## 10. Required tests

- **C:** `cargo check --manifest-path bindings/c/Cargo.toml --locked`; `cargo clippy --manifest-path bindings/c/Cargo.toml --locked --all-targets -- -D warnings`; `cargo test --manifest-path bindings/c/Cargo.toml --locked`; `bindings/c/scripts/generate-header.sh --check`; `bindings/c/scripts/check-exports.sh`; `bindings/c/scripts/run-c-tests.sh`; `bindings/c/scripts/parity.sh`.
- **Node:** in `bindings/node`, run `pnpm install --frozen-lockfile`, `pnpm build`, `pnpm test`, and `pnpm typecheck`; verify generated declaration drift and the Node 22/24/26 runtime smoke workflow.
- **Python:** in `bindings/python`, run `cargo test --no-default-features`, build and install a real wheel, and run `python -m pytest -v`; run the manual native release qualification for all five documented targets.
- Re-run all three RustSec audits after updates. The root workspace suite remains required for shared library changes but is not a substitute for the binding checks.

## 11. Required verification commands

Run audit commands from the repository root:

```bash
cargo audit --file bindings/c/Cargo.lock
cargo audit --file bindings/node/Cargo.lock
cargo audit --file bindings/python/Cargo.lock
```

C workflow commands from the repository root:

```bash
cargo check --manifest-path bindings/c/Cargo.toml --locked
cargo clippy --manifest-path bindings/c/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path bindings/c/Cargo.toml --locked
bindings/c/scripts/generate-header.sh --check
bindings/c/scripts/check-exports.sh
bindings/c/scripts/run-c-tests.sh
bindings/c/scripts/parity.sh
```

Node workflow commands from `bindings/node/`:

```bash
pnpm install --frozen-lockfile
pnpm build
git diff --exit-code -- index.js index.d.ts stegoeggo.d.ts
pnpm test
pnpm typecheck
```

Python workflow commands from `bindings/python/`:

```bash
cargo test --no-default-features
maturin build --release --out dist-ci
python -m pip install --force-reinstall dist-ci/stegoeggo-*.whl
python -m pytest -v
```

Also run `./scripts/check.sh` from the repository root if shared source changes. Record the C/Node/Python compatibility workflow run links and the manual Python qualification run link; do not edit workflow triggers to force these checks into required CI.

## 12. Documentation updates

After the PyO3 upgrade, update version-specific statements in `architecture/bindings-python.md` and `.skills/bindings/SKILL.md`. Update the language-bindings roadmap status table, this plan's closure record, and `plans/registry.md` when evidence is complete. No public API documentation, C ABI contract, or release policy change is expected; update those only if implementation evidence reveals an externally visible change, and stop for a maintainer decision before making such a change.

## 13. Acceptance criteria (externally observable)

- `cargo audit` reports no advisory for each of the three binding lockfiles; Python resolves to PyO3 0.29.0 or a later release confirmed patched for both recorded advisory IDs.
- C and Node binding compatibility workflows pass with no unexplained ABI/header/declaration drift.
- Python Rust tests, wheel build/install, pytest suite, and five-target wheel qualification pass at the supported Python 3.11 abi3 floor.
- The Python API, Node API, C ABI v1, panic-unwind policy, exact root dependency, and platform support commitments are unchanged.
- No binding dependency is added to the root workspace and no binding-specific tool is added to required CI.

## 14. Stop conditions (report rather than improvise)

Stop if the patched PyO3 line cannot build on Rust 1.89, requires changing the supported Python ABI/platform matrix, exposes a C ABI or Python/Node API change, or cannot be validated on the existing wheel qualification matrix. Stop on any advisory requiring an unapproved exception, or if a lockfile update entails unrelated major migrations. Report the dependency path and evidence; do not hide it with a waiver.

## 15. Closure evidence required

Create `plans/closure/language-bindings/011-status.md` only after implementation. Include baseline/head SHAs; exact PyO3 and binding direct/transitive version changes; all three audit results and advisory-database date; exact commands in section 11; C/Node/Python workflow links; manual Python five-target qualification result; generated-file/ABI checks; docs and registry updates; and every skipped or failed signal. A successful Linux-only build is not cross-platform wheel qualification.

## 16. Handoff notes

Root workspace remediation is separately tracked in `plans/implementation/maintenance-quality/006-root-workspace-dependency-security-refresh.md`; it owns the root `Cargo.lock` and root advisories. The binding locks must be audited independently because they are outside the root workspace. This plan changes dependency maintenance state only; it does not authorize package publication or changes to binding public contracts.
