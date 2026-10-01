# Release and Distribution Milestone 002 — Eggpack Producer Adoption and Second-Consumer Qualification

Status: ready for handoff

Repository baseline: `8c89e8cb1677a355d639ca1ead93c3dd2587e317`

Source roadmap:

- `plans/subsystems/release-distribution-roadmap.md`

Long-term requirements:

- `plans/000-long-term-specification.md#4-cli-invariants`
- `plans/000-long-term-specification.md#5-release-invariants`
- `plans/001-terminology-and-domain-model.md#distribution`

Applicable ADRs:

- `plans/adrs/ADR-0004-verified-self-update.md`

Upstream Eggpack plan:

- `eggstack/eggpack: plans/implementation/ecosystem-adoption/002-stegoeggo-direct-release-adoption-and-second-consumer-qualification.md`
- registered on Eggpack at `58b0ef81827c2a9e6a51f973ad123dac80b0397d`

Upstream qualified baseline:

- `eggstack/eggpack@32a0903936fcc283863e0bfb86151b13b4d75ce9` before M002 planning registration;
- implementation must use one exact reviewed Eggpack commit and never a floating branch/tag.

Primary class: capability

## 1. Objective

Adopt Eggpack as StegoEggo's producer authority for the existing five-target
native CLI release while preserving all StegoEggo-owned release, installer,
and self-update policy.

Replace the handwritten `.github/workflows/release-binaries.yml` matrix and
its duplicate producer tables with checked-in `release/eggpack/`
configuration plus a generated, drift-gated Eggpack workflow.

Use StegoEggo as Eggpack's second independent direct-binary consumer. The
migration must demonstrate that the same producer model proven by eggsact
works without adding StegoEggo-specific production logic to Eggpack.

## 2. Why this milestone is ready

Hard/interface dependencies are satisfied:

- Eggpack Ecosystem M001 (eggsact) is closed with real five-target live release
  evidence;
- Eggpack Build M006 and CI M003e/M003f/M003g are closed;
- Eggpack M003h merged/reconciled that qualified producer state onto
  `main`;
- StegoEggo Plan 105 closed the existing 0.4.2 five-target binary release,
  including native Windows and Linux glibc-2.17 evidence;
- current StegoEggo public binary target/asset/install/update contracts are
  documented and testable.

The existing Release-Distribution M001 / flat Plan 106 remains blocked only on
an **operational release event**: no ordinary stable B newer than 0.4.2 has
been published. It is not a code dependency for M002 implementation.

M002 may therefore implement now. The next ordinary stable release can be
shared as:

- M002's first live Eggpack-produced StegoEggo release; and
- M001/Plan 106's real public 0.4.2 -> B self-update proof.

Do not publish a throwaway stable version just to generate either milestone's
evidence.

## 3. Current implementation evidence

### Current native release workflow

`.github/workflows/release-binaries.yml` is manual-dispatch and builds:

| Target | Runner | Build | Asset |
|---|---|---|---|
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` | cargo-zigbuild, glibc 2.17 | `stegoeggo-x86_64-unknown-linux-gnu` |
| `aarch64-unknown-linux-gnu` | `ubuntu-24.04-arm` | cargo-zigbuild, glibc 2.17 | `stegoeggo-aarch64-unknown-linux-gnu` |
| `x86_64-apple-darwin` | `macos-15-intel` | native Cargo | `stegoeggo-x86_64-apple-darwin` |
| `aarch64-apple-darwin` | `macos-latest` | native Cargo | `stegoeggo-aarch64-apple-darwin` |
| `x86_64-pc-windows-msvc` | `windows-latest` | native Cargo | `stegoeggo-x86_64-pc-windows-msvc.exe` |

Each target currently runs:

- exact `stegoeggo X.Y.Z` version smoke;
- `--help`;
- protect canonical PNG fixture;
- inspect produced PNG;
- checksum-sidecar generation.

The attach job copies the two product installers and uses
`gh release upload ... --clobber` against an already-created release.

### Existing producer duplication

Producer facts currently appear in:

- `.github/workflows/release-binaries.yml`;
- `scripts/release-targets.txt`;
- `scripts/release-binary-preflight.sh`;
- `scripts/release-check-assets.sh`;
- `packaging/install.sh`;
- `packaging/install.ps1`;
- `stegoeggo-cli/src/update.rs::TARGETS`.

M002 removes duplicate **producer authority**, but installer/updater runtime
policy remains product-owned and self-contained.

### Current toolchain ambiguity

The legacy Linux workflow pins Zig 0.13.0 but installs cargo-zigbuild through
an unversioned action. That is not a deterministic producer contract.

Eggpack's already-qualified provisioning uses Zig 0.14.1 plus
cargo-zigbuild 0.23.3 with exact official archive digests. M002 adopts that
pair unless implementation-time qualification proves a StegoEggo-specific
incompatibility.

### Current release sequencing

`RELEASING.md` currently requires:

1. manual crates.io carrier -> library -> CLI publication;
2. exact tag;
3. manually create a GitHub Release;
4. dispatch binary workflow;
5. workflow attaches assets with clobber authority.

M002 preserves steps 1-2 and human publication authority, but replaces steps
3-5 with Eggpack draft staging:

1. manual crates.io publication;
2. exact immutable tag;
3. manual Eggpack workflow dispatch;
4. Eggpack creates/reuses a draft, never clobbers mismatched bytes;
5. maintainer reviews and explicitly publishes.

### Baseline CI

At reviewed baseline `8c89e8c`, standard `CI` run 36874105954 is green.
Python/Node binding workflows have independent failures at this same commit;
they are pre-existing, separately-owned experimental binding work and are not
silently reclassified as M002 defects. Any failure caused by M002-touched
release files must still be fixed before M002 closure.

## 4. Invariants that must not regress

- all three Rust crates remain lockstep versioned;
- crates.io remains stable-version authority;
- publication order remains carrier -> root library -> CLI;
- version tag is created only after intended crates.io publication succeeds;
- release automation never publishes crates;
- release automation never publishes the GitHub draft;
- tags are never force-moved by CI;
- five public native binary asset names remain unchanged;
- each binary retains one matching `.sha256` sidecar;
- public `install.sh` / `install.ps1` semantics remain StegoEggo-owned;
- unsupported-target and exact-404 are the only installer/updater Cargo
  fallback cases;
- checksum, identity/version, TLS/network, and non-404 server failures remain
  hard failures;
- `stegoeggo update` remains crates.io-authoritative and Eggup-transaction
  backed;
- Linux native CLI compatibility floor remains glibc 2.17;
- all five published candidates execute on matching native runners;
- no Python/Node publication behavior is absorbed into this native CLI
  milestone;
- no new Eggpack schema/API is introduced solely for StegoEggo.

## 5. Scope

### In

- add static `release/eggpack/` producer configuration;
- generate/check in Eggpack native release workflow;
- deterministic cross-tool provisioning;
- five-target native qualification;
- StegoEggo-specific bounded candidate validator;
- Linux GLIBC-symbol ceiling validation;
- product-wrapper presentation plus generated exact installers;
- refactor release preflight/asset audit to consume Eggpack authority;
- add updater/installer parity guards;
- add exact Eggpack drift guard;
- update release docs to draft-first binary assembly;
- execute the first normal post-cutover stable release when available;
- share that release event with existing Plan 106 A->B evidence.

### Explicitly out

- automatic crates.io publication;
- changing release cadence;
- updater manifest consumption / Eggup interoperability redesign;
- Python wheel migration;
- Node addon migration;
- C ABI work;
- code signing/notarization;
- new native release targets;
- changing image/protection behavior;
- changing the SHA-256 trust model;
- creating a fake stable B solely for tests;
- weakening Eggpack no-clobber semantics.

## 6. Required production changes

### 6.1 Static Eggpack configuration

Add:

```text
release/eggpack/
  distribution.toml
  pack.toml
  build-bindings.toml
  qualification-bindings.toml
  consumer-validators.json
  installer-presentation.json
  install-policy.toml
  github-template.json
  github-policy.json
  workflow-shape.json
```

Model the five current direct targets only.

Use aliases:

- `linux-x64`;
- `linux-arm64`;
- `macos-x64`;
- `macos-arm64`;
- `windows-x64`.

No static file may contain a future tag/source SHA/artifact digest.

### 6.2 Build bindings

For every target:

- package: `stegoeggo-cli`;
- binary: `stegoeggo`;
- selector: direct.

Do not request new Eggpack feature-flag support. The CLI package currently
declares:

```toml
[features]
default = ["signatures"]
signatures = ["stegoeggo/signatures", "stegoeggo/detached-manifest"]
```

so a default Cargo build preserves the predecessor's distributed
`signatures` feature set. Add a release-contract guard so future feature
drift fails visibly.

### 6.3 Pack/toolchain policy

Linux:

- strategy: `CargoZigbuild`;
- qualification: `Native`;
- matching native build/qualification architecture;
- Rust: stable;
- glibc floor: 2.17;
- Zig: 0.14.1;
- cargo-zigbuild: 0.23.3;
- exact official Zig archive digest supplied through Eggpack GitHub policy.

macOS/Windows:

- strategy: `NativeCargo`;
- qualification: `Native`;
- matching native host;
- Rust: stable.

All five remain required.

The 0.13.0 -> 0.14.1 Zig move is intentional deterministic-provisioning
normalization. It is accepted only after the candidate matrix and GLIBC
validation pass.

### 6.4 Qualification binding

Use one bounded core smoke per target:

```text
version
```

with finite timeout/stdout/stderr limits.

Core smoke proves the exact candidate executes. Product semantic validation is
separate below.

### 6.5 Product consumer validator

Add a bounded repository script, recommended:

`scripts/smoke-release-binary.py PATH`

It must receive only the exact Eggpack candidate path and use checked-in source
fixtures/config.

At minimum it must:

1. determine expected workspace release version;
2. require `PATH version == "stegoeggo X.Y.Z"`;
3. run `PATH --help`;
4. create a temporary output directory;
5. run `protect tests/fixtures/conformance/canonical/canonical_policy_only.png
   --output <tmp>/protected.png --rights-policy prohibited-ai-ml-training
   --preset legal-notice`;
6. run `inspect` on that output;
7. run `verify` when valid for the generated evidence;
8. remove/close temporary state;
9. enforce bounded subprocess time/output and return nonzero on any mismatch.

For Linux candidates, the same validator or a companion validator must inspect
the candidate's required GLIBC symbol versions and fail if the maximum
requirement exceeds GLIBC_2.17.

Use a bounded locally available ELF inspection tool. If `readelf`/the chosen
tool is missing, fail the Linux release validator rather than silently
dropping ABI evidence.

This is deliberately StegoEggo-owned. Eggpack native smoke does not become a
claim that the declared compatibility floor has independently been proven.

### 6.6 Installer presentation

Ship product wrappers unchanged in semantic ownership:

- `packaging/install.sh` -> `install.sh`;
- `packaging/install.ps1` -> `install.ps1`.

Add Eggpack generated exact installers:

- `install-exact.sh`;
- `install-exact.ps1`.

Keep:

- latest vs exact-version selection;
- destination/PATH behavior;
- unsupported/404 Cargo fallback;
- fatal checksum/network/identity errors.

### 6.7 Generated workflow

Generate `.github/workflows/release-binaries.yml` from static Eggpack config.

Initial trigger remains manual `workflow_dispatch` with an exact existing
release tag.

The workflow must:

- resolve runtime release identity from the exact tag + checked-out commit;
- install Eggpack from an exact immutable revision;
- build all five targets;
- qualify exact candidates natively;
- run StegoEggo consumer validators;
- aggregate/finalize;
- stage a draft GitHub Release;
- isolate `contents: write` to the staging job;
- contain no publish, tag-mutation, release-delete, or clobber command.

Do not leave a second active legacy binary workflow.

## 7. Ordered work packages

### WP1 — Re-review baseline and freeze authority map

Before edits:

- re-fetch StegoEggo and Eggpack current heads;
- re-review the current five-target matrix, assets, installers, updater
  `TARGETS`, CLI feature defaults, release docs, and release scripts;
- record any material change from baseline `8c89e8c`;
- choose the exact Eggpack pin.

Stop if current code materially invalidates this plan.

### WP2 — Author static Eggpack config and parity fixture

Create `release/eggpack/`.

Before activating a workflow, prove:

- exactly five target aliases;
- exact public asset names;
- Windows `.exe` convention;
- package/bin binding;
- default feature `signatures`;
- glibc floor;
- target/runner architecture;
- public wrapper paths.

Run Eggpack resolution/rendering in a scratch/non-triggering location.

### WP3 — Add product validator and ABI gate

Implement the bounded candidate validator and tests for:

- exact version identity;
- help;
- protect/inspect/verify;
- timeout/nonzero handling;
- temporary-file cleanup;
- GLIBC parser positive and >2.17 rejection fixtures on Linux.

Do not rely solely on shell fragments embedded in generated YAML.

### WP4 — Reconcile release contract scripts

Refactor:

- `scripts/release-binary-preflight.sh`;
- `scripts/release-check-assets.sh`;
- `scripts/release-targets.txt` as appropriate.

Producer facts should derive from Eggpack config/projection.

Retain product-owned checks:

- version lockstep;
- exact internal dependency versions;
- CLI default feature contract;
- installer/updater mapping parity;
- release-version/tag consistency;
- post-build checksum/content checks.

Post-cutover asset audit recognizes the 15-asset Eggpack native CLI inventory,
not the predecessor 12-asset inventory.

### WP5 — Add immutable drift guard

Add a release-specific CI check that:

1. installs Eggpack from the exact pinned Git SHA;
2. verifies the tool identity;
3. runs `eggpack ci check`;
4. runs StegoEggo release-contract parity checks.

It must fail generated workflow drift. Caching is optional; the immutable pin
is not.

### WP6 — Cut over active workflow

Only after WPs 2-5 pass:

- replace active `.github/workflows/release-binaries.yml` with generated
  output;
- verify no duplicate active native binary workflow;
- inspect generated permissions and commands;
- run ordinary CI plus release-specific checks.

The predecessor remains available in Git history.

### WP7 — Update release documentation

Update at minimum:

- `RELEASING.md`;
- `docs/installation.md`;
- `SUPPORT.md` where workflow/tool details are described;
- release-distribution roadmap and registry as status changes.

Document the new binary release sequence:

```text
publish crates manually
  -> verify crates.io visibility
  -> create/push exact tag
  -> dispatch generated Eggpack workflow
  -> inspect complete draft
  -> maintainer publishes
  -> run public installer/update evidence
```

Do not imply Eggpack publishes crates or the GitHub release.

### WP8 — Pre-live qualification

Before the next stable release:

- render/check zero drift;
- run full release checks;
- run installer/updater rehearsals;
- run Linux x86-64 CargoZigbuild with final pins;
- ensure AArch64/macOS/Windows generated jobs match intended native hosts;
- where practical, perform two Windows release builds and compare digests to
  detect rerun nondeterminism before live staging.

A known Windows nondeterminism finding never authorizes `--clobber`.

### WP9 — First normal Eggpack-produced stable release

When the next ordinary stable B > 0.4.2 is ready:

1. execute normal pre-release checks;
2. publish carrier/library/CLI manually and confirm registry propagation;
3. create/push the exact immutable tag;
4. manually dispatch generated Eggpack workflow;
5. require all five build/qualification/consumer-validator jobs to pass;
6. inspect the complete 15-asset draft and staging receipt;
7. rerun same tag to exercise reconciliation/no-clobber;
8. maintainer publishes only after inspection;
9. download/audit public assets;
10. smoke exact/latest public installers.

Do not consume an otherwise unnecessary version for this test.

### WP10 — Share B with existing M001 / Plan 106 updater proof

After B is public/updater-ready, execute flat Plan 106 against the real
production endpoints:

- isolated public 0.4.2 binary as A;
- `stegoeggo update` -> B;
- exact checksum/identity/version proof;
- functional B smoke;
- B already-current no-op;
- deterministic updater regression script.

Record M001 evidence in `plans/106-status.md`; record M002 producer adoption in
its own closure record.

## 8. Failure, cancellation, restart, contention semantics

- Static config/render failure: no public mutation; fix before workflow cutover.
- Build/qualification/consumer validation failure: staging is blocked.
- Draft staging partial failure: retain draft for inspection; rerun must
  reconcile exact matching assets and refuse differing bytes.
- Same-name/different-digest: fail closed; never clobber. Determine whether the
  cause is consumer nondeterminism or producer defect.
- Crates.io partial publication: follow existing immutable-version recovery;
  do not republish accepted bytes.
- Tag must never move to conceal a failed release.
- Human publication is outside Eggpack; a failed draft workflow remains draft.
- Concurrent dispatches for the same tag rely on generated workflow
  concurrency/reconciliation policy.
- If implementation discovers a missing generic Eggpack capability, stop and
  register it upstream before continuing the cutover.
- If an unrelated Python/Node binding lane remains red, record it separately;
  do not weaken native release checks or falsely claim repo-wide green.

## 9. Compatibility and migration

Public binary names stay byte-for-byte identical as names, preserving:

- existing installer URLs;
- updater URL construction;
- user documentation;
- latest-release links.

Public installer semantics and updater semantics stay product-owned.

The public release inventory intentionally grows from 12 to 15 files because
Eggpack adds:

- `release-manifest.json`;
- `install-exact.sh`;
- `install-exact.ps1`.

That inventory expansion is part of producer evidence, not a breaking
executable naming change.

Rollback before the first public Eggpack-backed release is a Git revert of the
workflow/config cutover. After a successful public Eggpack-backed release,
changing producer authority again requires a corrective plan; do not silently
restore the legacy clobber workflow.

## 10. Required tests

Add/retain coverage for:

- Eggpack config resolves all five targets;
- contract asset names exactly match installer/updater expectations;
- `stegoeggo-cli` default feature set remains exactly `signatures`;
- updater `TARGETS` matches Eggpack published targets;
- public installer target names match contract;
- generated workflow drift is detected;
- exactly one staging job has write permission;
- no generated publish/tag/clobber command;
- bounded candidate validator happy path;
- validator wrong version/nonzero/timeout cases;
- Linux GLIBC <=2.17 accepted and >2.17 rejected;
- asset audit accepts exactly expected 15-asset post-cutover inventory and
  rejects missing/extra/misnamed assets;
- installer/updater deterministic rehearsal tests remain green.

## 11. Required verification commands

Minimum implementation gate:

```bash
./scripts/check.sh
cargo test -p stegoeggo-cli --all-features
./scripts/test-release-installers.sh
./scripts/test-release-updater.sh
./scripts/release-check.sh --stage=pre --allow-dirty
python3 scripts/smoke-release-binary.py <native-candidate>   # or equivalent focused test harness
eggpack ci check ...
```

Also run as applicable:

```bash
cargo +1.89.0 check -p stegoeggo-stego --locked
cargo +1.89.0 check -p stegoeggo --locked --all-features
cargo +1.89.0 check -p stegoeggo-cli --locked --all-features
cargo deny check licenses
cargo deny check advisories
bash -n packaging/install.sh
```

PowerShell installer parsing/tests remain required on Windows evidence.

At live closure record the exact GitHub workflow/job conclusions for all five
targets and staging.

## 12. Documentation updates

Update:

- `RELEASING.md`;
- `docs/installation.md`;
- `SUPPORT.md` if release runner/toolchain wording changes;
- root README only if release-distribution description becomes stale;
- `plans/subsystems/release-distribution-roadmap.md`;
- `plans/registry.md`.

Documentation must distinguish:

- crates.io authority;
- Eggpack producer authority;
- Eggup updater transaction ownership;
- product wrapper policy;
- human publication authority;
- declared glibc floor vs independently inspected final ELF requirement.

## 13. Acceptance criteria

Implementation/cutover is complete when:

- static Eggpack configuration exists and resolves exactly five targets;
- generated workflow is checked in and is the only active native binary
  producer;
- exact immutable Eggpack pin is used;
- five public binary names are unchanged;
- CLI default `signatures` feature equivalence is guarded;
- Zig/cargo-zigbuild are deterministic and final Linux candidates retain the
  GLIBC_2.17 ceiling;
- candidate version/help/protect/inspect(/verify) validation gates all five
  targets;
- updater and installer mappings remain parity-gated but product-owned;
- release preflight/asset audit no longer depend on a handwritten producer
  matrix;
- generated workflow stages draft-only with one writer and no clobber;
- ordinary/release-specific CI passes.

Full operational closure additionally requires the next ordinary stable
release to prove:

- five-target live generated run;
- complete 15-asset draft;
- staging receipt;
- human publication only after inspection;
- public exact/latest installer smoke;
- rerun exact reuse or correctly owned/fail-closed nondeterminism finding;
- no unresolved medium-or-higher release regression.

Closure path:

`plans/closure/release-distribution/002-status.md`.

## 14. Stop conditions

Stop and report rather than improvise if:

- current Eggpack interfaces cannot express the five-target shape;
- preserving CLI feature semantics requires a new generic build-flag feature;
- deterministic cross-tool pins raise the Linux runtime floor;
- product updater/install policy would have to move into Eggpack;
- generated release requires automatic publication or tag mutation;
- same-name/different-digest behavior would need clobbering;
- Python/Node publication becomes entangled with native CLI adoption;
- current repository changes materially invalidate the reviewed baseline.

## 15. Closure evidence required

Create `plans/closure/release-distribution/002-status.md` containing:

- final StegoEggo implementation SHA;
- exact Eggpack pin;
- before/after authority map;
- static config inventory;
- five-target parity matrix;
- cross-tool versions/digests;
- GLIBC-symbol-floor evidence;
- product-validator matrix;
- installer/updater parity evidence;
- generated workflow drift/permission evidence;
- implementation verification commands/results;
- live release version/source/tag;
- live workflow run and per-job results;
- first staging receipt and 15-asset inventory;
- rerun reuse/no-clobber result;
- publication actor/evidence;
- public installer smoke;
- relationship to M001/Plan 106 A->B update evidence;
- unresolved findings by severity and owner;
- rollback notes.

## 16. Handoff notes

This is intentionally a second-consumer adoption, not an Eggpack redesign.

Prefer copying the proven **shape** of eggsact's `release/eggpack/` setup,
then replacing product ids, package/bin names, product validator, installer
policy, and release docs with StegoEggo's contracts.

Do not copy eggsact-specific MCP semantics or its Windows reproducibility
finding as assumptions. Test StegoEggo's actual binaries.

The key new evidence this consumer should contribute is:

- independent second-repo producer adoption;
- StegoEggo protect/inspect/verify candidate semantics;
- explicit final-ELF GLIBC ceiling validation;
- composition with an existing Eggup-based self-update path;
- proof that the next normal release can satisfy both producer-adoption and
  real A->B updater evidence without coupling their ownership.
