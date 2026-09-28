# Language Bindings Milestone 005 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/language-bindings/005-node-binding-foundation-qualification.md`
Source subsystem roadmap: `plans/subsystems/language-bindings-roadmap.md#m005--nodejs-binding`
Repository baseline reviewed: `32f4f09`
Implementation commits: `0cfc390` — feat: add Node.js napi-rs binding foundation and qualification (M005); `abc0354` — fix: stage release-node artifacts in the napi output dir

## 1. Executive finding

M005 implementation is landed and all locally executable qualification
evidence is green: the isolated napi-rs v3 leaf under `bindings/node/`
projects the canonical `ProtectionRequest` / `RightsPolicy` byte API as a
typed Promise-based Node surface with `bigint` seeds, structured
`ERR_STEGOEGGO_*` rejections, generated declarations verified by a
compile-only consumer contract, and byte-identical deterministic parity
with Rust in both directions (including a `u64::MAX` vector). The
lightweight `node-binding` CI signal is green on the implementation
tree, including the generated-file drift check and runtime smokes on
Node 22, 24, and 26. The five-target `release-node` qualification is
green on the exact implementation SHA `abc0354`: five native
build+smoke jobs plus the assembly/pack/clean-install job (run
`36488540671`). No acceptance criterion remains outstanding; M005
closes.

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Isolated leaf, outside root workspace | `bindings/node/Cargo.toml` has local `[workspace]`; `cargo metadata --workspace` from root does not list `stegoeggo-node`; no napi deps in root/carrier/CLI graphs |
| napi-rs v3, no compat-mode | `napi 3.13.0`, `napi-derive 3.6.9`, `napi-build 2.5.0` in `bindings/node/Cargo.lock`; no `compat-mode` feature anywhere |
| Node-API 6 justified by BigInt | `napi6` feature only; `withSeed(bigint)` / `seed: bigint \| null` round-trip `u64::MAX` exactly (§4 parity) |
| Engines floor documented | `engines.node >= 22.13.0` in `bindings/node/package.json`, README, SUPPORT |
| Node 22/24/26 load and test the addon | `node-binding` run `36486934874`: one Linux x86_64 build reused for smoke on 22, 24, 26 — all green |
| Promise-based off-thread protect/verify | `AsyncTask` workers in `src/tasks.rs`; event-loop liveness + concurrency tests in `test/operations.test.mjs` |
| Input copied before worker dispatch | `owned_bytes` copy at the sync boundary; caller-mutation-after-call test |
| Buffer and Uint8Array accepted; Buffer out | Tested for every public operation |
| `u64` seeds use bigint, >MAX_SAFE_INTEGER parity | `18446744073709551615n` metadata-only output byte-identical to the Rust oracle (§4) |
| Structured error codes/properties, stable | 9-code matrix below; `instanceof Error` + field assertions in `test/errors.test.mjs`; no message parsing |
| No secret key leakage | Secret-material sweep over errors, reports, request inspection, declarations, and outputs |
| Rust↔Node parity both directions | `test/parity.test.mjs`: Node→CLI verify, CLI→Node verify, oracle byte equality ×3 |
| Deterministic output matches Rust | `cmp`-equal bytes for metadata-only, HMAC marker, and `u64::MAX` vectors |
| Declarations compile against a consumer fixture | `test/types/contract.ts` + `pnpm typecheck` (strict, `skipLibCheck: false`) |
| Declaration drift check | CI step `git diff --exit-code -- index.js index.d.ts` after `pnpm build`, green |
| Five targets build + same-arch smoke | `release-node` run `36487115740` (see §4) |
| napi-cross GNU Linux (glibc 2.17 floor) | `--use-napi-cross` on both Linux build jobs; documented as addon floor only |
| Platform dirs + loader metadata assembled/audited | Assembly job: `create-npm-dirs` + `artifacts`, 5-dir/5-artifact assertion, per-platform `os`/`cpu`/`libc` audit, loader require-string audit |
| Clean local pack/install smoke | Root tarball (7 files, no `.node`/secrets) installs clean; smoke passes against the installed copy with the platform package stood in explicitly |
| No npm version published or consumed | No publish/pre-publish step in any workflow; install sources are lockfile + local tarball only |
| Docs distinguish tested support from ABI possibility | SUPPORT matrix rows Configured; README states tested lines vs out-of-scope runtimes |
| `./scripts/check.sh` unchanged and green | No Node steps added; green on the implementation commit |

Structured error-code/property matrix:

| `code` | Structured fields |
|---|---|
| `ERR_STEGOEGGO_INVALID_CONFIG` | — |
| `ERR_STEGOEGGO_INVALID_FORMAT` | — |
| `ERR_STEGOEGGO_ENCODE_DECODE` | — (includes `ImageTruncated`) |
| `ERR_STEGOEGGO_METADATA` | — |
| `ERR_STEGOEGGO_STEGANOGRAPHY` | — |
| `ERR_STEGOEGGO_INSUFFICIENT_CAPACITY` | `required`, `available` |
| `ERR_STEGOEGGO_VERIFICATION` | — |
| `ERR_STEGOEGGO_RESOURCE_LIMIT` | `resource` + `size`/`limit` (`input_bytes`), `width`/`height`/`maxWidth`/`maxHeight` (`dimensions`), `kind`/`count`/`limit` (`container`, `metadata`, `verification_budget`) |
| `ERR_STEGOEGGO_INTERNAL` | safe non-exhaustive fallback |

`InsufficientCapacity` is not reachable through the public best-effort
API (small carriers degrade with `LSB_CAPACITY_SKIPPED` /
`DCT_CAPACITY_INSUFFICIENT` warnings instead), so the JS-side projection
is exercised through `toPublicError` with a synthetic DTO while the
canonical Rust variant mapping is covered by the binding's own Rust unit
tests — the accommodation the plan explicitly allows for unreachable
variants. `verification_budget` is likewise covered by a binding-side
projection test.

## 3. Production implementation evidence

New isolated package `bindings/node/` (31 files, all additive; no root,
carrier, or CLI file modified):

- `Cargo.toml` (crate `stegoeggo-node` 0.4.2, `rust-version 1.89`,
  `publish = false`, `crate-type cdylib`, exact `stegoeggo = "=0.4.2"`
  path dependency with default features off, `panic = "unwind"` in
  dev/release) + `Cargo.lock`; `build.rs` (`napi-build 2.5.0` setup).
- `src/`: `lib.rs` (`stegoeggoVersion`, bounded sync `detectFormat`),
  `enums.rs` (14 string enums), `notice.rs` (`RightsNotice`,
  `ProcessingOptions`), `limits.rs` (`ResourceLimits` + immutable builder),
  `request.rs` (immutable `ProtectionRequest` builder over canonical
  values), `report.rs` (typed report projections), `error.rs` (`ErrorCode`
  + `NativeError` DTO + Rust-side variant mapping),
  `numeric.rs` (validated number→Rust narrowing), `tasks.rs` (four
  `AsyncTask` workers with `owned_bytes` copies and `catch_unwind` →
  `ERR_STEGOEGGO_INTERNAL`).
- `src/bin/oracle.rs`: dev-only byte oracle for parity (same canonical
  calls, explicit seed/timestamp CLI args). Not part of the shipped
  surface; covered by the binding clippy/test gates.
- `stegoeggo.js` / `stegoeggo.mjs`: the only hand-written runtime layer;
  converts native outcome DTOs into Promise operations rejecting with a
  real `Error` (`code` + structured fields, napi statuses normalized into
  the stable family). `stegoeggo.d.ts`: public type contract re-exporting
  generated declarations.
- `index.js` / `index.d.ts`: napi-rs generated loader + declarations,
  committed and drift-checked.
- `test/`: 64 unit/integration tests (`smoke`, `request`, `operations`,
  `errors`), 7 parity tests, `types/contract.ts` compile-only fixture,
  `tsconfig.json` (strict, `skipLibCheck: false`, `@types/node 22`).
- `scripts/smoke.mjs`: self-contained smoke (builds its own tiny PNG,
  asserts version/loader-target/Buffer/Promise/round-trip; parameterizable
  via `STEGOEGGO_SMOKE_SPEC` for installed-package smoke).
- `.github/workflows/node-binding.yml`: lightweight binding CI.
- `.github/workflows/release-node.yml`: manual five-target qualification.
- `package.json`: unpublished identity `@eggstack/stegoeggo` 0.4.2,
  `packageManager pnpm@12.6.0`, five napi targets, no publish scripts.

Public API inventory: `RightsNotice`, `ProcessingOptions`, `ResourceLimits`,
`ResourceLimitsBuilder`, `ProtectionRequest`, `HiddenMarkerMode`,
`RightsPolicy`, `DmiValue`, `ImageOutputFormat`, `MetadataUpdatePolicy`,
`ProtectionPreset`, `AuthenticationMode`, `ProtectionWarning`,
`ExecutionReport`, `ResourceUsage`, `VerificationStatus`,
`EvidenceStrength`, `VerificationReport` (+ nested report objects),
`protect`, `protectWithWarnings`, `protectWithReport`, `verify`,
`detectFormat`, `stegoeggoVersion`, `toPublicError`. No deprecated
`ProtectionLevel` / `ProtectionContext` / `EvidenceProfile` surface is
exported at runtime or in declarations.

## 4. Verification executed (exact commands + results)

Binding Rust gate (from the repository root):

```bash
cargo check --manifest-path bindings/node/Cargo.toml --locked        # clean
cargo clippy --manifest-path bindings/node/Cargo.toml --locked --all-targets -- -D warnings  # clean
cargo test --manifest-path bindings/node/Cargo.toml --locked         # 54 passed
```

Node gate (`cd bindings/node`, pnpm 12.6.0, Node v22.23.3 locally):

```bash
pnpm install --frozen-lockfile   # clean
pnpm build                       # addon + regenerated index.js/index.d.ts
pnpm test                        # 64 passed, 0 failed
pnpm typecheck                   # strict, clean
pnpm smoke                       # ok (version 0.4.2)
```

Parity (CLI built first, exactly as the plan requires):

```bash
cargo build -p stegoeggo-cli     # from the repository root
cd bindings/node
STEGOEGGO_CLI=../../target/debug/stegoeggo pnpm test:parity   # 7 passed
```

Byte-equality vectors (all `cmp`-identical Node vs `oracle` output):
metadata-only (seed 4242), HMAC hidden marker (seed 7, 32-byte key), and
`u64::MAX` (`18446744073709551615n`) metadata-only. Cross-direction:
Node→CLI `verify` exit 0 (metadata + HMAC with `--key`; wrong key exits
3), CLI→Node verify rights holder and `VERIFIED` marker.

Root gate: `./scripts/check.sh` green on the implementation commit
(includes `scripts/check-docs-contract.sh`: 5 release targets valid).
`check.sh` gained no Node prerequisite.

Remote qualification:

- `node-binding` run `36486934874` on SHA `0cfc390` — **success**:
  `node-binding-check` (build, drift check, 64 tests, typecheck) plus
  `smoke Node 22`, `smoke Node 24`, `smoke Node 26` reusing the same
  built addon, all green.
  `https://github.com/eggstack/stegoeggo/actions/runs/36486934874`
- `release-node` run `36488540671` on SHA `abc0354` — **success**
  (all six jobs green):
  - `build linux-x86_64 addon` (job `109151275680`, `ubuntu-24.04`,
    napi-cross): smoke passed on Node 22, 24, and 26
    (3× `stegoeggo package smoke ok`); artifact
    `node-addon-linux-x86_64` → `stegoeggo.linux-x64-gnu.node`
    (757,517 bytes).
  - `build linux-aarch64 addon` (job `109151275718`,
    `ubuntu-24.04-arm`, napi-cross): Node 22 smoke passed; artifact
    `node-addon-linux-aarch64` → `stegoeggo.linux-arm64-gnu.node`
    (731,335 bytes).
  - `build macos-x86_64 addon` (job `109151275408`, `macos-15-intel`,
    native): Node 22 smoke passed; artifact
    `node-addon-macos-x86_64` → `stegoeggo.darwin-x64.node`
    (739,171 bytes).
  - `build macos-arm64 addon` (job `109151275777`, `macos-14`,
    native): Node 22 smoke passed; artifact
    `node-addon-macos-arm64` → `stegoeggo.darwin-arm64.node`
    (707,579 bytes).
  - `build windows-x86_64 addon` (job `109151275641`,
    `windows-2022`, native): Node 22 smoke passed; artifact
    `node-addon-windows-x86_64` → `stegoeggo.win32-x64-msvc.node`
    (693,664 bytes).
  - `assemble package` (job `109155499067`, `ubuntu-24.04`):
    `create-npm-dirs` + `artifacts` collected all five addons;
    `package layout audit ok` (5 dirs, per-platform `os`/`cpu`/`libc`
    and root loader require-strings verified); `pnpm pack` produced
    `eggstack-stegoeggo-0.4.2.tgz` with exactly the 7 intended files;
    `clean local pack/install smoke ok`
    (`stegoeggo package smoke ok (version 0.4.2)` against the
    installed copy).
  `https://github.com/eggstack/stegoeggo/actions/runs/36488540671`
- Superseded attempt: `release-node` run `36487115740` on SHA `0cfc390` —
  all five native build+smoke jobs green, assembly failed on artifact
  staging in the wrong directory (workflow bug, fixed in `abc0354`).
  Qualification evidence is cited from the `abc0354` run only; the
  native addons are byte-identical across the two runs (no Rust/package
  change between them).

Local package-assembly rehearsal (single-target, on the implementation
commit): `create-npm-dirs` produced all five platform dirs with correct
`name`/`version`/`os`/`cpu`/`libc`/`main`; the one buildable artifact was
staged the way `artifacts` stages it; `pnpm pack` produced an 18.4K
tarball containing exactly the 7 intended files (no `.node`, no secrets,
no build junk); a clean `npm install <tarball>` in an empty directory
plus the explicitly stood-in platform package passed the
installed-package smoke (`holder: clean install smoke, status: VERIFIED`).

No `npm publish`, `pnpm publish`, or `napi pre-publish` was invoked
anywhere; the npm registry was never contacted except for the initial
`pnpm add -D @types/node` dependency installation.

## 5. Invariant review

- `#![forbid(unsafe_code)]` holds for root and carrier; the binding adds
  no `unsafe` and no new root/carrier/CLI dependency.
- `bindings/node` is outside the root Cargo workspace with its own
  lockfile; root release builds keep `panic = "abort"`, the addon uses
  `panic = "unwind"`.
- Every protection operation delegates to `ProtectionRequest` +
  `process_request_bytes*`; verification delegates to
  `verify_image_bytes_report`; metadata paths use encoded bytes only.
- A process abort during malformed-input testing never occurred; worker
  panics map to `ERR_STEGOEGGO_INTERNAL` via a narrow `catch_unwind`.
- napi-rs statuses leaking from argument conversion are normalized to
  `ERR_STEGOEGGO_INVALID_CONFIG` / `ERR_STEGOEGGO_INTERNAL`, never raw.
- napi-build resolved as `2.5.0`: the plan text says "napi-build v3" but
  the build crate for the napi-rs v3 line is versioned `2.x` on crates.io
  (latest stable `2.5.0` at implementation time). No `compat-mode` is
  used; the discrepancy is versioning-only, recorded here.
- C ABI work has not begun; no shared bindings crate was created.

## 6. Failure and recovery review

- Async worker errors reject the public Promise; partial bytes are never
  returned on failure (covered by rejection tests across all four
  operations).
- Input is fully copied before worker scheduling; the
  caller-mutation-after-call test proves the worker is unaffected.
- Concurrent same-tick operations keep per-task request/options
  ownership; seeds `[1n, 2n, 3n]` verified independently.
- `napi artifacts` collects `*.node` binaries from `--output-dir`
  (not the package root) and fails closed on a partial artifact set,
  **deleting target destinations including an already-collected local
  package-root `.node`** (observed locally, recorded in RELEASING.md
  §Node Native Artifacts): never run it without all five artifacts
  staged in the output dir; rebuild with `pnpm build` if the local
  addon goes missing. The first `release-node` assembly attempt
  (run `36487115740`) staged the five downloaded artifacts in the
  package root instead of the output dir and failed collection exactly
  this way; the workflow was corrected to stage into `artifacts/`.
- `stegoeggo.d.ts` initially used `export { X } from './index.js'`
  without a local `import`, so locally used names failed typecheck
  (TS2304 with no TS2307/TS2305 — re-export is not a local binding).
  Fixed with an explicit `import type` block; `typecheck` is strict with
  `skipLibCheck: false` to prevent recurrence.
- No automatic retry, no global mutable request/key state, no
  AbortSignal in M005 (listed exclusion).

## 7. Migration and compatibility review

New unpublished 0.x frontend: no existing Node consumer contract to
preserve. Compatibility-sensitive surface from here on: string enum
values, error codes, structured fields, Promise-vs-sync behavior, and
`bigint` seeds (never silently changed to `number`). Binding/package
version tracks the wrapped root version (0.4.2). No migration impact on
Rust, CLI, carrier, or Python consumers. M006 C ABI design must learn
from this closure; Node pre-designs nothing about C ownership.

## 8. Security review

MAC/HMAC key material: never in error messages/properties, reports,
request inspection (`macKey` is not an own property; `hasMacKey` is
boolean-only), generated declarations, or logs — swept by dedicated
tests including a fixed-secret scan and a declarations-file scan.
Output bytes scanned for key leakage. Resource limits are preserved
for untrusted inputs on both protect and verify paths. No `unsafe`
code added; `#![forbid(unsafe_code)]` untouched in root and carrier.

## 9. Documentation and operations

- `bindings/node/README.md`: install/build from source, Promise byte
  API, bigint contract, error codes, runtime support matrix,
  unpublished status, out-of-scope runtimes.
- `SUPPORT.md`: Node binding section with Node-API level, engine
  floor, tested lines, five-target matrix (all rows Qualified on run
  `36488540671`), glibc floor, lightweight CI pointer.
- `RELEASING.md`: manual Node artifact runbook, assembly commands,
  M005 non-publication rule, `napi artifacts` destructive-behavior
  warning, future-publication non-atomicity warning.
- Workflows `node-binding.yml` / `release-node.yml` carry header
  comments stating scope, triggers, and non-publication.

## 10. Unresolved findings (critical/high/medium/low)

None. Every required evidence item in plan §15 is recorded in §4 with a
passing result; the two investigation findings (napi-build 2.5.0
versioning, `napi artifacts` staging/destructive behavior) are
documented mitigations, not open defects.

## 11. Roadmap disposition

M005 is closed with all required evidence accepted. M006 C ABI design
becomes dependency-ready: its hard dependency (Python + Node closure)
is satisfied. No new subsystem-local milestone is required; C ABI work
starts with a fresh M006 implementation plan, which must consume the
ownership/error/panic lessons recorded in §5–§8.

## 12. Registry updates

- `plans/registry.md`: M005 `closing` → `closed`; subsystem current
  milestone notes M005 closed and M006 dependency-ready.
- `plans/subsystems/language-bindings-roadmap.md`: M005 row
  `closing` → `closed` with this closure record linked; M006 row notes
  the Python + Node closure dependency satisfied.
- `SUPPORT.md`: all five Node rows `Configured` → `Qualified` on
  `release-node` run `36488540671` (SHA `abc0354`).