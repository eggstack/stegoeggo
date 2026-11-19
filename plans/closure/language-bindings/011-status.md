# Language Bindings Milestone 011 — Closure Status

Status: conditionally closed
Source implementation plan: `plans/implementation/language-bindings/011-binding-dependency-security-refresh.md`
Source subsystem roadmap: `plans/subsystems/language-bindings-roadmap.md#12-milestone-status-table`
Repository baseline reviewed: `3d2388638b0d5ed3fb01b76f9b80b2a3f25746cb`
Implementation commit: `0b400cf20780e394f3a43d45987208dce4fd3eb8` — `security: refresh workspace and binding dependencies`.

## 1. Executive finding

All three excluded binding lockfiles have been refreshed and audited using RustSec data fetched on 2026-11-17. Python now requires PyO3 `0.29` and resolves to `0.29.3`; the Python Rust tests, native CPython 3.11 abi3 wheel build/install, and pytest suite pass. Local C and Node compatibility checks also pass, including the 90-symbol C contract and generated Node declaration drift check. The PyO3 architecture and binding guidance now describe the 0.29 line.

This milestone remains **conditionally closed**, not fully closed. On PR #3/SHA `0b400cf20780e394f3a43d45987208dce4fd3eb8`, the path-filtered C, Node, and Python workflows passed, but manual run [44166332867](https://github.com/eggstack/stegoeggo/actions/runs/44166332867) failed before builds because its two `dtolnay/rust-toolchain` steps omitted the required `toolchain` input. The follow-up candidate `7f5f6a1363ed78b05266f579467cb772e5fb05f7` pins Rust 1.89.0 in both steps. Its manual qualification retry [44175567864](https://github.com/eggstack/stegoeggo/actions/runs/44175567864) built four platform wheels and the sdist successfully, but GitHub canceled the macOS arm64 job because `macos-14-arm64-us-default` is unsupported; the dependent wheel smoke was skipped. This workflow follow-up changes both macOS arm64 entries to the documented `macos-15` runner; successful five-target requalification is still required. No merge, release, or publication has occurred.

## 2. Requirement-to-evidence matrix

| Plan requirement | Evidence |
|---|---|
| Refresh and independently audit C, Node, and Python locks | All three Cargo lockfiles regenerated; fresh `cargo audit` commands for each exited 0 on 2026-11-17 data (§4). |
| Upgrade PyO3 to a patched line | `bindings/python/Cargo.toml` now requires `0.29`; `bindings/python/Cargo.lock` resolves PyO3 `0.29.3`; Python tests and abi3 wheel install pass (§3–4). |
| Preserve C ABI and generated header | C Rust checks/tests pass; header drift check passes; export check confirms exactly 90 symbols; native C/C++ test script reports 174 checks and 0 failures; parity passes (§4–5). |
| Preserve Node behavior and declarations | Frozen install, build, 65 tests, typecheck, generated declaration diff check, and Node 26.10.0 smoke pass (§4). Node 22/24 local smoke was not run. |
| Preserve Python 3.11 abi3 behavior and GIL detachment | Rust tests pass on Rust 1.89; arm64 CPython 3.11 abi3 wheel builds and installs; pytest reports 77 passed and 1 intentionally skipped; existing `py.detach` calls are unchanged (§3–5). |
| Run path-filtered and five-target remote qualification | On PR #3, C/Node/Python workflows pass on SHA `0b400cf`; candidate `7f5f6a1` passes required Check and all path-filtered jobs. `release-python` run `44175567864` passes four wheel builds, sdist build, and sdist smoke, but macOS arm64 is cancelled because its `macos-14` runner configuration is unsupported; wheel smoke is skipped (§4, §10). |

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

The actual install used the exact arm64 wheel path and the final pytest run used a CI-like clean `PATH`. The root `./scripts/check.sh` also passed for the shared root-source adaptation; detailed root evidence is in M006. Remote qualification on PR #3/SHA `0b400cf20780e394f3a43d45987208dce4fd3eb8`: [C binding job](https://github.com/eggstack/stegoeggo/actions/runs/44165868979/job/134331629270), [Node binding job](https://github.com/eggstack/stegoeggo/actions/runs/44165868995/job/134331628649), [Python binding job](https://github.com/eggstack/stegoeggo/actions/runs/44165869735/job/134331634005), [required Check job](https://github.com/eggstack/stegoeggo/actions/runs/44165868955/job/134331630837), and [Eggpack drift/contract job](https://github.com/eggstack/stegoeggo/actions/runs/44165870536/job/134331634353) all passed. Node smoke jobs for Node 22, 24, and 26 also passed ([workflow run](https://github.com/eggstack/stegoeggo/actions/runs/44165868995)). The first manual [five-target `release-python` run](https://github.com/eggstack/stegoeggo/actions/runs/44166332867) failed before builds because both Rust setup steps lacked the required `toolchain` input; commit `7f5f6a1` added Rust 1.89.0 pins. Retry [44175567864](https://github.com/eggstack/stegoeggo/actions/runs/44175567864) successfully built the sdist and Linux x86_64, Linux aarch64, macOS x86_64, and Windows x86_64 wheels; its direct-pip-install sdist smoke passed. GitHub canceled the macOS arm64 wheel job before setup with `The configuration 'macos-14-arm64-us-default' is not supported`; the dependent wheel smoke matrix was skipped. Thus five-target wheel qualification is incomplete. The [current GitHub-hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) lists `macos-15` as a standard arm64 label; the follow-up workflow change now sets both matrix entries to `macos-15`, and a successful rerun remains outstanding.

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

- **Medium — five-target Python qualification rerun pending.** Required Check, C/Node/Python workflows, and Node 22/24/26 smokes passed on PR #3/SHA `7f5f6a1363ed78b05266f579467cb772e5fb05f7`. The prior run `44166332867` failed at missing Rust `toolchain` inputs; those are fixed in `7f5f6a1`. Retry `44175567864` built four platform wheels and the sdist and passed the sdist smoke, but GitHub canceled macOS arm64 before setup because `macos-14-arm64-us-default` is unsupported; the dependent wheel smoke matrix was skipped. The follow-up workflow change sets both arm64 matrix entries to the documented `macos-15` label. A complete rerun is required; no five-target or cross-platform qualification claim is made.
- **Low — 17 PyO3 deprecation warnings.** They concern automatic `FromPyObject` for `Clone` pyclasses and are non-fatal; they were not broadened into wrapper/API changes in this security refresh.
- **Low — optional Python CLI round-trip skipped.** It was skipped in the clean CI-like environment because the relevant optional CLI was not present; the full Python test result is 77 passed, 1 skipped.

No unresolved binding RustSec vulnerability is known from the final audits.

## 11. Roadmap disposition

Language-bindings M011 remains conditionally closed. Required Check, C/Node/Python workflows, Eggpack drift/contract, and Node 22/24/26 smokes passed on candidate `7f5f6a1`. The release-python retry built four wheel targets and the sdist and passed sdist smoke, but GitHub canceled macOS arm64 because the `macos-14` runner configuration is unsupported and skipped the dependent wheel-smoke matrix. The follow-up workflow change uses `macos-15` for both macOS arm64 matrix entries; a successful five-target rerun is still required before closure. M001–M010 remain closed and the completed Python → Node → C capability sequence is not reopened.

## 12. Registry updates

`plans/registry.md` records M011 as conditionally closed, links this record, and records the successful candidate C/Node/Python checks, repaired Rust toolchain setup, and the retry's canceled macOS arm64 job due to unsupported `macos-14`. The roadmap records the `macos-15` follow-up and the remaining five-target qualification gate. M011 is not listed as fully closed or as a recent unconditional closure. M006 is independently closed in maintenance-quality; no other subsystem status or blocker changed.
