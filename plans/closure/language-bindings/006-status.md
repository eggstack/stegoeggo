# Language Bindings Milestone 006 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/language-bindings/006-post-core-compatibility-corrective.md`
Source subsystem roadmap: `plans/subsystems/language-bindings-roadmap.md#m006--post-core-compatibility-corrective-and-requalification`
Repository baseline reviewed: `d24b37767a44fc530ebb3e8efed420f7b0a5d731`
Implementation commits: `e6c1e2b` — fix: reconcile Python/Node bindings with current canonical core (language M006 implementation); `d24b377` — fix: add Python carrier-limit projection tests; tidy needless returns

## 1. Executive finding

Python and Node are reconciled with the current canonical Rust surface
and freshly requalified on their documented five-target matrices at the
final SHA. The carrier-level `Error::ResourceLimitExceeded(String)` maps
explicitly into each language's existing resource-limit family as
`resource = "carrier"` with the canonical message preserved and no
invented fields; the Python metadata-limit `kind` assertion now checks
the stable semantic contract (present, non-empty, Rust-provided) instead
of freezing the old `"tEXt field"` wording; the Node metadata-only
rights source/channels expectation matches current canonical output
(`xmp` / `['pngXmp', 'pngText']`); verification status is projected
verbatim with no wrapper reinterpretation. Remote CI
(`36922556633`), `python-binding` (`36922556590`), fresh
`release-python` (`36922890804`) and `release-node` (`36922895760`)
qualification are all green on `d24b377`. No acceptance criterion
remains outstanding; M006 closes. M005 closure stays untouched as
historical evidence for its own SHA.

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Explicit Python mapping for `ResourceLimitExceeded(String)` | `bindings/python/src/lib.rs:140`: `ResourceLimitError` with `resource = "carrier"`, canonical message, no fabricated numerics (§3) |
| Explicit Node mapping for `ResourceLimitExceeded(_)` | `bindings/node/src/error.rs:237`: `ERR_STEGOEGGO_RESOURCE_LIMIT` + `resource = "carrier"`, all unavailable fields `None` (§3) |
| No invented structured facts for the carrier case | Python unit test asserts `size`/`limit` attributes absent; JS test asserts `size`/`limit`/`kind`/`count` undefined (§4) |
| Unknown future variants still fall back safely | Wildcard arms retained (`Internal` / generic `StegoEggoError`); synthetic-unknown tests pass in both suites (§4) |
| Metadata `kind` is descriptive context, not an enum | Python test asserts `isinstance(err.kind, str) and len > 0`; both READMEs document branching on `resource`, not `kind` wording (§3) |
| No wrapper reinterpretation of verification status | No status-mapping code changed; unprotected → `NotFound` and metadata-only expectations pass verbatim in both suites (§4) |
| Python/Node verification parity with Rust | `NotFound` for unprotected images agrees across Rust/Python/Node; parity suite 7/7 (§4) |
| Error-family parity table (§5E) | Six-row table satisfied: input_bytes, dimensions, container, metadata, verification_budget, carrier — covered by language-local tests (§4) |
| No secret leakage | Secret sweeps pass in both suites; carrier test asserts no secret string in the error (§4, §8) |
| Fresh five-target Python qualification | `release-python` run `36922890804` on `d24b377`: 5 wheels + sdist build, 5 native smokes + sdist direct-install smoke, all green (§4) |
| Fresh five-target Node qualification | `release-node` run `36922895760` on `d24b377`: 5 native addon build+smokes, assembly/pack/clean-install green (§4) |
| No PyPI/npm publication | No publish step in any workflow; none invoked locally (§8) |
| M005 closure untouched | `plans/closure/language-bindings/005-status.md` unmodified; new evidence cited only here (§9) |

## 3. Production implementation evidence

`e6c1e2b` (executable binding changes; no root/carrier/CLI file touched):

- `bindings/python/src/lib.rs` (+5): explicit
  `RustError::ResourceLimitExceeded(_)` arm projecting
  `ResourceLimitError` with `resource = "carrier"` and the canonical
  message; unavailable numeric attributes left absent.
- `bindings/python/tests/test_errors.py` (1 line): metadata-limit
  assertion relaxed from `err.kind == "tEXt field"` to present-and-non-empty.
- `bindings/python/README.md` (+5): documents the additive
  `resource="carrier"` value and the descriptive-`kind` contract.
- `bindings/node/src/error.rs` (+86): explicit
  `RustError::ResourceLimitExceeded(_)` arm (`resource = "carrier"`,
  all structured numerics `None`, message preserved) plus two Rust
  projection unit tests (carrier maps to resource-limit/carrier;
  structured prior variant preserved).
- `bindings/node/test/errors.test.mjs` (+30): JS public-error test for
  the carrier projection (code, `resource`, message, undefined
  numerics) plus fallback tests retained.
- `bindings/node/test/operations.test.mjs` (4 lines): metadata-only
  expectation updated to current canonical `rights.source == 'xmp'`
  and `channels == ['pngXmp', 'pngText']`.
- `bindings/node/README.md` (+5): same `carrier`/`kind` contract note
  as the Python README.

`d24b377` (test-only, excluded from release builds by `#[cfg(test)]`):

- Two Rust projection tests in `bindings/python/src/lib.rs`
  (carrier → `ResourceLimitError`/`carrier` with absent `size`/`limit`;
  `InputTooLarge` → `input_bytes` preserved) plus two
  `needless_question_mark` cleanups and the PyO3
  `Python::initialize` + `Python::attach` modernization required by
  clippy. No executable-code semantic change: the release wheel built
  from `e6c1e2b` and `d24b377` sources is equivalent.

## 4. Verification executed (exact commands + results)

Root gate (repository root, clean tree at `d24b377`):

```bash
./scripts/check.sh   # exit 0; closes with `./scripts/check-docs-contract.sh`: "Documentation contracts valid: 5 release targets"
```

Python gate:

```bash
cargo clippy --manifest-path bindings/python/Cargo.toml --locked --all-targets -- -D warnings  # clean
python3 -m pytest bindings/python/tests -q   # 71 passed (local, CPython 3.14, installed 0.4.2 wheel)
```

Remote: `python-binding` run `36922556590` on `d24b377` — **success**
(`python-binding-check`: wheel build + install + full pytest).
`https://github.com/eggstack/stegoeggo/actions/runs/36922556590`

Node gate (from the repository root / `bindings/node`, pnpm 12.6.0):

```bash
cargo check --manifest-path bindings/node/Cargo.toml --locked   # clean
cargo clippy --manifest-path bindings/node/Cargo.toml --locked --all-targets -- -D warnings  # clean
cargo test --manifest-path bindings/node/Cargo.toml --locked    # 56 passed (54 M005 + 2 carrier projection)
cd bindings/node
pnpm install --frozen-lockfile  # clean
pnpm build                      # addon + regenerated index.js/index.d.ts
git diff --exit-code -- index.js index.d.ts   # clean (no drift)
pnpm test                       # 65 passed (64 M005 + 1 carrier projection)
pnpm typecheck                   # strict, clean
cargo build -p stegoeggo-cli    # from the repository root
STEGOEGGO_CLI=../../target/debug/stegoeggo pnpm test:parity  # 7 passed
```

Remote lightweight CI: `CI` run `36922556633` on `d24b377` —
**success** (`Check` job).
`https://github.com/eggstack/stegoeggo/actions/runs/36922556633`
`node-binding` has no run on `d24b377` by design: its path filter
(`bindings/node/**`, `src/**`, carrier, manifests, fixtures) excludes
the python-only `d24b377` diff, so there was nothing new to check. Last
`node-binding` success is run `36919087763` on `e6c1e2b`
(`node-binding-check` + Node 22/24/26 smokes, all green), and no
Node-affecting change landed after it.

Fresh five-target qualification on the final SHA `d24b377`:

- `release-python` run `36922890804` — **success** (all 12 jobs green):
  5 wheel builds (Linux x86_64/aarch64, macOS x86_64/arm64, Windows
  x86_64) + sdist build, 5 same-native smokes + sdist direct
  `pip install <tarball>` smoke in a clean venv off `sys.path`.
  Artifacts: `python-wheel-linux-x86_64`, `python-wheel-linux-aarch64`,
  `python-wheel-macos-x86_64`, `python-wheel-macos-arm64`,
  `python-wheel-windows-x86_64`, `python-sdist` (no registry upload).
  `https://github.com/eggstack/stegoeggo/actions/runs/36922890804`
- `release-node` run `36922895760` — **success** (all 6 jobs green):
  5 native addon build+smokes (Linux x86_64 with napi-cross glibc 2.17
  floor + Node 22/24/26 smokes; Linux aarch64, macOS x86_64/arm64,
  Windows x86_64 with native Node smokes) + `assemble package`
  (`create-npm-dirs`/`artifacts` audit, local pack tarball, clean
  install/load smoke). Artifacts: `node-addon-linux-x86_64`,
  `node-addon-linux-aarch64`, `node-addon-macos-x86_64`,
  `node-addon-macos-arm64`, `node-addon-windows-x86_64` (no registry
  upload). `napi pre-publish` never invoked.
  `https://github.com/eggstack/stegoeggo/actions/runs/36922895760`

Before/after disposition: at the verification-conformance M006 baseline
the two binding suites were the independent detectors of the core
false positive (Python 70 passed / 1 failed on `kind` wording; Node
63 pass / 1 fail on the rights-source label — see
`plans/closure/verification-conformance/006-status.md` §4). After this
corrective both suites are fully green against the same core
(71 and 65 passed respectively), with the `Invalid`-vs-`NotFound`
contract fixed in Rust rather than masked in wrappers.

Limitation (low, §10): the two new Python `#[cfg(test)]` Rust unit
tests do not run in any workflow (`python-binding` runs pytest only)
and `cargo test --lib` for the PyO3 extension does not link on stock
macOS (missing libpython symbols — pre-existing platform limitation,
unrelated to this change). They compile under
`clippy --all-targets`, passed in the implementation session's Linux
run, and their behavior is covered by the pytest suite.

## 5. Invariant review

- Foreign bindings do not reinterpret canonical verification status;
  Python and Node expose the same status semantics (parity 7/7).
- Existing error classes/codes stable; `resource="carrier"` is purely
  additive inside the existing resource-limit family.
- Resource exhaustion never degrades to internal/generic merely
  because Rust gained a variant; the wildcard fallback remains for
  genuinely unknown futures.
- No unavailable structured facts invented; secret material never in
  messages/properties/repr/JSON (swept by tests).
- `kind` stays a descriptive Rust-provided string; no new binding enum.
- CPU-detachment/GIL behavior, Node input-copy/thread-safety, Python
  file-helper atomicity: unchanged (no code touched in those paths).
- Binding crates remain outside the root workspace with own lockfiles;
  `./scripts/check.sh` gained no Python/Node prerequisite.
- No PyPI/npm publication; M005 closure history untouched.

## 6. Failure and recovery review

- Cancelled/failed early qual attempts on the pre-final SHA
  (`release-python` `36920313225`, `release-node` `36920319221`,
  both failure in under a minute on `e6c1e2b` superseded dispatches)
  are not cited as evidence; only the green final-SHA runs above count.
- Partial platform success would not qualify the matrix; both final
  runs are green on every documented row plus assembly/sdist jobs.
- The carrier projection preserves the Rust message verbatim rather
  than parsing it, so future message rewording cannot silently
  reclassify; misclassification would surface as a test failure, not
  a silent fallback, because the mapping is on the Rust variant, not
  the string.

## 7. Migration and compatibility review

Additive-only change for binding consumers: one new `resource` value
(`"carrier"`) inside the existing `ResourceLimitError` /
`ERR_STEGOEGGO_RESOURCE_LIMIT` family, plus a documented
clarification that `kind` is descriptive context. Consumers branching
on `resource`/class/code are unaffected; consumers asserting
`kind == "tEXt field"` must follow the relaxed contract (the one
in-tree case was updated). No version bump or publication is tied to
this corrective; version policy stays with the next StegoEggo source
release. No migration impact on Rust/CLI consumers.

## 8. Security review

MAC/HMAC key material: never in carrier messages (Rust message is a
free-form resource notice, asserted secret-free in tests),
properties, reports, request inspection, declarations, or logs —
swept by the dedicated secret tests in both suites, all passing. No
`unsafe` added anywhere; `#![forbid(unsafe_code)]` untouched in root
and carrier. No registry credentials exist in Actions for PyPI/npm;
no `twine upload`, `npm publish`, `pnpm publish`, or
`napi pre-publish` was invoked locally or in CI.

## 9. Documentation and operations

- `bindings/python/README.md` and `bindings/node/README.md`: carrier
  `resource="carrier"` + descriptive-`kind` contract documented.
- `SUPPORT.md`: Python wheel matrix re-qualified on
  `release-python` run `36922890804` (SHA `d24b377`); Node addon
  matrix re-qualified on `release-node` run `36922895760` (SHA
  `d24b377`); evidence pointers moved from `003`/`005-status.md`
  (now historical) to this record.
- `RELEASING.md`: unchanged (qualification process itself did not change).
- Workflows unchanged; no new CI prerequisites.

## 10. Unresolved findings (critical/high/medium/low)

None blocking. One low process finding, recorded for M007+ hygiene:
the Python binding's Rust `#[cfg(test)]` unit tests have no workflow
runner (`python-binding.yml` runs pytest only; `release-python.yml`
builds wheels and runs pytest smokes). Consider a Linux
`cargo test -p stegoeggo-python` step with libpython available, or
keep projection coverage in pytest. Not a correctness gap: behavior
is covered by the green pytest suites on every matrix row.

## 11. Roadmap disposition

Language-bindings M006 is closed with all required evidence accepted.
The M007 C ABI design hard dependency (two currently-green
foreign-runtime clients) is now satisfied: M007 becomes
dependency-ready and may be planned. No new subsystem-local milestone
is required by this closure.

## 12. Registry updates

- `plans/registry.md`: language-bindings M006 implementation plan
  `active` → `closed` with this closure record linked; subsystem
  current milestone notes M006 closed and M007 dependency-ready; M007
  blocker row removed; recent-closure entry added.
- `plans/subsystems/language-bindings-roadmap.md`: §12 M006 row
  `active` → `closed` with this closure record linked; M007 row
  blocker cleared (implementation plan not yet written).
- `SUPPORT.md`: wheel/addon matrices re-pointed to the fresh
  final-SHA qualification runs (see §9).
