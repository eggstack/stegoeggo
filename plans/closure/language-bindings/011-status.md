# Language Bindings Milestone 011 — Closure Status

Status: conditionally closed
Source implementation plan: `plans/implementation/language-bindings/011-binding-dependency-security-refresh.md`
Source subsystem roadmap: `plans/subsystems/language-bindings-roadmap.md#12-milestone-status-table`
Repository baseline reviewed: `3d2388638b0d5ed3fb01b76f9b80b2a3f25746cb`
Implementation commits: none; candidate changes remain uncommitted in the working tree at the baseline HEAD, so there is no candidate SHA.

## 1. Executive finding

All three excluded binding lockfiles have been refreshed and audited using RustSec data fetched on 2026-11-17. Python now requires PyO3 `0.29` and resolves to `0.29.3`; the Python Rust tests, native CPython 3.11 abi3 wheel build/install, and pytest suite pass. Local C and Node compatibility checks also pass, including the 90-symbol C contract and generated Node declaration drift check. The PyO3 architecture and binding guidance now describe the 0.29 line.

This milestone is **conditionally closed**, not fully closed. The path-filtered `.github/workflows/c-binding.yml`, `node-binding.yml`, and `python-binding.yml` workflows and the manual five-target `.github/workflows/release-python.yml` qualification have not run for an immutable candidate SHA. Local macOS arm64 evidence is not cross-platform qualification. No commit, push, PR, workflow dispatch, or publication was authorized or performed. Preserve this status until those candidate-specific workflow results are recorded.

## 2. Requirement-to-evidence matrix

| Plan requirement | Evidence |
|---|---|
| Refresh and independently audit C, Node, and Python locks | All three Cargo lockfiles regenerated; fresh `cargo audit` commands for each exited 0 on 2026-11-17 data (§4). |
| Upgrade PyO3 to a patched line | `bindings/python/Cargo.toml` now requires `0.29`; `bindings/python/Cargo.lock` resolves PyO3 `0.29.3`; Python tests and abi3 wheel install pass (§3–4). |
| Preserve C ABI and generated header | C Rust checks/tests pass; header drift check passes; export check confirms exactly 90 symbols; native C/C++ test script reports 174 checks and 0 failures; parity passes (§4–5). |
| Preserve Node behavior and declarations | Frozen install, build, 65 tests, typecheck, generated declaration diff check, and Node 26.10.0 smoke pass (§4). Node 22/24 local smoke was not run. |
| Preserve Python 3.11 abi3 behavior and GIL detachment | Rust tests pass on Rust 1.89; arm64 CPython 3.11 abi3 wheel builds and installs; pytest reports 77 passed and 1 intentionally skipped; existing `py.detach` calls are unchanged (§3–5). |
| Run path-filtered and five-target remote qualification | Not run: candidate SHA/PR ref was not authorized or created. `.github/workflows/c-binding.yml`, `node-binding.yml`, and `python-binding.yml`, plus manual five-target `.github/workflows/release-python.yml`, remain the explicit closure condition (§10). |

## 3. Production implementation evidence

- `bindings/c/Cargo.lock`, `bindings/node/Cargo.lock`, and `bindings/python/Cargo.lock` were regenerated independently. Their manifests remain isolated from the root workspace; the exact local `stegoeggo = "=0.4.2"` dependency, `default-features = false`, and panic-unwind profiles are unchanged.
- `bindings/python/Cargo.toml` moves the requirement from PyO3 `0.27` to `0.29`; Cargo resolves PyO3 `0.29.3`. The existing `py.detach` behavior, `abi3-py311`, and `extension-module` feature remain intact.
- Node's compatible napi-rs selections include `napi 3.16.2` and `napi-derive 3.7.1`. No generated `.js` or declaration change was needed.
- `architecture/bindings-python.md` and `.skills/bindings/SKILL.md` now identify the PyO3 0.29 line. No C ABI, Node API, Python public API, package version, workflow trigger, or publication policy changed.

## 4. Verification executed (exact commands + results)

RustSec audits, from the repository root:

```text
cargo audit --file bindings/c/Cargo.lock       PASS; exit 0, 2026-11-17 advisory data
cargo audit --file bindings/node/Cargo.lock    PASS; exit 0, 2026-11-17 advisory data
cargo audit --file bindings/python/Cargo.lock  PASS; exit 0, 2026-11-17 advisory data
```

C checks, from the repository root, using Rust 1.89.0 and cbindgen 0.29.4:

```text
cargo check --manifest-path bindings/c/Cargo.toml --locked
                                                     PASS
cargo clippy --manifest-path bindings/c/Cargo.toml --locked --all-targets -- -D warnings
                                                     PASS
cargo test --manifest-path bindings/c/Cargo.toml --locked
                                                     PASS; 27 tests passed
bindings/c/scripts/generate-header.sh --check       PASS; no header drift
cargo build --manifest-path bindings/c/Cargo.toml --locked --release
                                                     PASS
bindings/c/scripts/check-exports.sh                 PASS; exact 90-symbol contract
bindings/c/scripts/run-c-tests.sh                   PASS; 174 checks, 0 failures
bindings/c/scripts/parity.sh                        PASS
```

Node checks, from `bindings/node/`, with Node 26.10.0:

```text
pnpm install --frozen-lockfile                       PASS
pnpm build                                            PASS
git -C ../.. diff --exit-code -- bindings/node/index.js bindings/node/index.d.ts bindings/node/stegoeggo.d.ts
                                                     PASS; no generated-file drift
pnpm test                                             PASS; 65 tests
pnpm typecheck                                        PASS
node scripts/smoke.mjs                               PASS; Node 26.10.0 only
```

Python checks, from `bindings/python/`, using Rust 1.89.0 and native arm64 `/usr/local/bin/python3.11` (3.11.9):

```bash
PY311=/usr/local/bin/python3.11
export RUSTUP_TOOLCHAIN=1.89.0
export PYO3_PYTHON="$PY311"
cargo test --manifest-path Cargo.toml --locked --no-default-features
maturin build --release --out dist-ci
"$PY311" -m venv ../../target/m011-py311-venv
../../target/m011-py311-venv/bin/python -m pip install --upgrade pip pytest
../../target/m011-py311-venv/bin/python -m pip install --force-reinstall \
  dist-ci/stegoeggo-0.4.2-cp311-abi3-macosx_11_0_arm64.whl
PATH="../../target/m011-py311-venv/bin:/usr/bin:/bin:/usr/sbin:/sbin" \
  ../../target/m011-py311-venv/bin/python -m pytest -v
```

Results: Rust tests and wheel build/install passed; pytest reported **77 passed, 1 intentionally skipped**.

The actual install used the exact arm64 wheel path and the final pytest run used a CI-like clean `PATH`. The root `./scripts/check.sh` also passed for the shared root-source adaptation; detailed root evidence is in M006. No GitHub workflow run links exist yet because no candidate ref was created.

## 5. Invariant review

- C ABI v1's generated header, exact 90-symbol inventory, error/ownership contracts, and Rust–C parity remain unchanged.
- Node generated declarations and public runtime surface remain unchanged; the local smoke was on Node 26.10.0 only.
- Python remains CPython 3.11+ abi3 with `extension-module`, public surface unchanged, and blocking Rust calls still use `py.detach`.
- Binding packages remain excluded from the root workspace and are not added to `scripts/check.sh`; panic-unwind and exact root-library dependency constraints remain intact.
- No automatic publishing, CI topology change, or binding API redesign was introduced.

## 6. Failure and recovery review

- An initial Python build selected an x86_64 interpreter on the arm64 host and failed to link; validation was rerun with native arm64 `/usr/local/bin/python3.11` and succeeded.
- A broad wheel glob also selected a stale x86_64 wheel; installing the exact newly built arm64 wheel succeeded.
- An exploratory pytest run inherited an unrelated globally installed CLI whose `verify ... -o` behavior did not match this binding's optional CLI round-trip. The final CI-like clean-`PATH` run avoided that unrelated executable and passed (77 passed, 1 intentionally skipped). The optional CLI round-trip remains skipped in that environment; no binding test failure remains unresolved.
- PyO3 0.29.3 emits 17 non-fatal deprecation warnings concerning `Clone` pyclasses' automatic `FromPyObject` implementation. They do not fail the build/tests and are recorded as a low finding (§10).

## 7. Migration and compatibility review

The only binding manifest constraint change is Python's PyO3 line. Rust 1.89, Python 3.11 abi3, Node package contracts, C ABI v1, package version `0.4.2`, and platform commitments are unchanged. Local checks validate the current macOS arm64 environment; cross-platform compatibility must not be inferred until the candidate workflows complete.

## 8. Security review

Fresh RustSec audits for all three binding lockfiles exited successfully on 2026-11-17 data. The Python PyO3 advisories RUSTSEC-2026-0176 and RUSTSEC-2026-0177 are remediated by the 0.29.3 resolution. No advisory exception or allowlist change was introduced. The refreshed C and Node lockfiles have no unresolved advisory reported by their audits. Remote artifact qualification remains outstanding and is not treated as security evidence here.

## 9. Documentation and operations

`architecture/bindings-python.md` and `.skills/bindings/SKILL.md` now describe PyO3 0.29 and `py.detach`. This record, the implementation plan, language-bindings roadmap, and registry reflect conditional closure. No generated header/declaration update, workflow-trigger change, publication, or root-workspace binding integration occurred.

## 10. Unresolved findings (critical/high/medium/low)

- **Medium — remote candidate qualification outstanding.** `.github/workflows/c-binding.yml`, `node-binding.yml`, and `python-binding.yml` have not run on an immutable candidate SHA. The Node 22/24 smoke rows are also not represented by the local Node 26.10.0 smoke.
- **Medium — Python five-target wheel qualification outstanding.** The manual `.github/workflows/release-python.yml` matrix was not dispatched. Local evidence is one macOS arm64 CPython 3.11 abi3 wheel only; no cross-platform claim is made.
- **Low — 17 PyO3 deprecation warnings.** They concern automatic `FromPyObject` for `Clone` pyclasses and are non-fatal; they were not broadened into wrapper/API changes in this security refresh.
- **Low — optional Python CLI round-trip skipped.** It was skipped in the clean CI-like environment because the relevant optional CLI was not present; the full Python test result is 77 passed, 1 skipped.

No unresolved binding RustSec vulnerability is known from the final audits.

## 11. Roadmap disposition

Language-bindings M011 is conditionally closed pending candidate-specific C/Node/Python GitHub workflows and the manual five-target Python wheel qualification. M001–M010 remain closed and the completed Python → Node → C capability sequence is not reopened. The language-bindings roadmap remains active only for M011's outstanding evidence.

## 12. Registry updates

`plans/registry.md` records M011 as conditionally closed, links this record, and names the remote workflows and five-target Python qualification as the remaining evidence. The language-bindings roadmap status table and current-state summary match. M011 is not listed as fully closed or as a recent unconditional closure. M006 remains an independent maintenance-quality milestone; no other subsystem status or blocker changed.
