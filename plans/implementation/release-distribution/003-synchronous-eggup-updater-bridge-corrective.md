# Release and Distribution Milestone 003 — Synchronous Eggup Updater Bridge Corrective

Status: ready / not started

Repository baseline: `c75132a09a0079c857a5b241dfbe254747ddb84d`

Source roadmap:

- `plans/subsystems/release-distribution-roadmap.md`

Related milestones/evidence:

- Release-Distribution M001 / flat Plan 106 — blocked on the next ordinary stable B and a green updater rehearsal;
- Release-Distribution M002 — conditionally closed at `plans/closure/release-distribution/002-status.md`;
- M002 implementation: `3b96fae753d003d5bf916141ff777c4102428593`;
- Eggup current reviewed baseline: `eggstack/eggup@e336b3203183aa84d174e7d7bfa087ce4b61b077`;
- Eggup adapter: `crates/eggup-eggfetch/src/lib.rs`.

Primary class: corrective

## 1. Finding

M002 closure reproduced a pre-existing panic in
`./scripts/test-release-updater.sh` at the untouched pre-M002 baseline and at
the current tree.

The ownership review shows this is a StegoEggo updater-integration defect, not
an Eggup transport defect.

StegoEggo currently does:

```text
run_update()
  -> create Tokio current-thread runtime
  -> runtime.block_on(run_update_async())
       -> eggup_get_bytes(...)
            -> EggfetchTransport::fetch_metadata(...)
                 -> EggfetchTransport::block_on(...)
                      -> create/block_on private Tokio runtime
```

The same nesting is reachable later through:

```text
run_update_async()
  -> update_to(...).await
       -> seam_download(...)
            -> EggfetchTransport::fetch_artifact(...)
                 -> EggfetchTransport::block_on(...)
```

Eggup's sync adapter explicitly owns a private current-thread runtime and states
that async consumers must call the synchronous seam through their own
`spawn_blocking` policy if they remain async.

StegoEggo does not need an async updater at all:

- `update_to` is declared `async` but contains no `.await`;
- `run_update_async` only performs synchronous work plus
  `update_to(...).await`;
- Eggup prepare/validate/commit is synchronous;
- release selection and Cargo fallback are synchronous.

The smallest correct fix is therefore to remove the unnecessary outer Tokio
runtime and make the updater path synchronous end to end.

## 2. Objective

Eliminate the nested-runtime panic without changing release selection,
transport policy, fallback rules, Eggup transaction semantics, installer
behavior, or the public CLI surface.

After M003, the updater must call the synchronous Eggup acquisition seam from
ordinary synchronous StegoEggo code.

## 3. Ownership

### StegoEggo owns

- whether its CLI update command is synchronous or async;
- the call context around `eggup-eggfetch`;
- crates.io release selection;
- exact asset/checksum URL construction;
- Cargo fallback policy;
- candidate/version UX and diagnostics.

### Eggup owns

- the synchronous `AcquisitionTransport` seam;
- its private Eggfetch runtime bridge;
- bounded transport policy;
- verified transaction/ownership/rollback mechanics.

No Eggup production change is authorized by this corrective unless new
evidence demonstrates the documented synchronous contract itself is defective.

## 4. Invariants

The fix MUST preserve:

- crates.io stable-version authority;
- prerelease/yanked filtering;
- exact GitHub version-tagged asset names;
- unsupported-target and exact binary-404 as the only Cargo fallback cases;
- checksum-sidecar failure as fatal after a binary is found;
- strict HTTPS downgrade denial;
- explicit environment proxy handling;
- current connect/total/body bounds;
- no external curl dependency in `stegoeggo update`;
- exact Eggup ownership/integrity/version validation;
- backup/rollback/recovery dispositions;
- Windows self-replacement behavior;
- already-current update as a no-download/no-replacement path;
- public CLI command and output semantics except removal of the panic.

## 5. Required implementation

### 5.1 Remove the unnecessary production async wrapper

Refactor:

```rust
async fn update_to(...)
async fn run_update_async(...)
pub(crate) fn run_update() { runtime.block_on(...) }
```

into a synchronous path equivalent to:

```text
run_update()
  -> parse current version
  -> eggup_get_bytes(registry)
  -> resolve latest stable
  -> update_to(current, latest)

update_to(...)
  -> synchronous seam_download / sidecar fetch
  -> synchronous Eggup transaction
  -> synchronous fallback / replacement
```

Do not add `spawn_blocking` merely to preserve an unnecessary async shape.

### 5.2 Audit Tokio ownership

After the production updater becomes synchronous, audit direct StegoEggo CLI
uses of Tokio.

If the direct `tokio` dependency exists only for the removed production
runtime plus retained test helpers:

- either move the minimum required Tokio features to `dev-dependencies`; or
- refactor those retained test-only direct-Eggfetch helpers so the direct
  dependency can be removed entirely.

Do not remove Tokio if another current production CLI path genuinely requires
it. Record the audit result in closure evidence.

### 5.3 Preserve test-only policy harness separation

The legacy direct-`eggfetch_core` helpers retained under `#[cfg(test)]`
remain policy tests, not production acquisition.

They may use a test runtime provided it is constructed outside another Tokio
runtime. Do not reintroduce the production nested-runtime shape through shared
helpers.

## 6. Regression tests

Add focused coverage that would have caught this defect.

At minimum:

1. a structural/source test proving production `run_update` no longer creates
   or blocks on a Tokio runtime;
2. a focused updater test that exercises registry acquisition through the
   Eggup seam from the same synchronous call context used by production;
3. a supported-target artifact path test through `seam_download` that reaches
   the Eggup seam without a nested-runtime panic;
4. already-current remains no-op after registry selection;
5. exact 404 still permits Cargo fallback only at the binary lookup boundary;
6. checksum 404/mismatch, 5xx, timeout, malformed proxy, and wrong candidate
   remain hard failures;
7. `./scripts/test-release-updater.sh` completes without panic.

Prefer deterministic local HTTP fixtures for regression tests. Do not require a
new public release to prove the runtime bridge fix.

## 7. Failure and recovery semantics

No error category is intentionally changed.

The corrective only removes an invalid execution context. Existing
`UpdateError` mapping, Eggup transaction disposition, destination ownership,
rollback, and Cargo fallback behavior stay authoritative.

A transport/transaction failure must still return an ordinary bounded error;
it must never be converted into a panic.

## 8. Compatibility and migration

No public CLI/API/schema or release-asset change is intended.

If the direct production Tokio dependency becomes removable, that is an
internal dependency-footprint cleanup and must not change command behavior.

This corrective does not consume a release version and does not modify the
Eggpack-generated release workflow/configuration.

## 9. Verification

Required minimum:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo test -p stegoeggo-cli --all-features
./scripts/test-release-updater.sh
./scripts/release-check.sh --stage=pre --allow-dirty
./scripts/check.sh
git diff --check
```

Also:

- run MSRV 1.89 checks for the CLI;
- run the release drift guard / `eggpack ci check` to prove the producer
  cutover did not drift;
- require hosted standard CI;
- if dependency placement changes, inspect `cargo tree -p stegoeggo-cli`
  and record the delta.

## 10. Acceptance criteria

M003 closes only when:

- production updater execution is synchronous around the synchronous Eggup seam;
- no production nested Tokio runtime can occur on registry, asset, or sidecar
  acquisition;
- `update_to` no longer carries an empty async abstraction;
- `./scripts/test-release-updater.sh` passes;
- deterministic regression tests cover the old failure shape;
- updater fallback/security/transaction invariants remain unchanged;
- ordinary CI and release drift guard pass;
- no medium-or-higher updater-runtime finding remains.

Closure record:

`plans/closure/release-distribution/003-status.md`.

## 11. Downstream disposition

Closing M003 removes the code blocker for the shared next-release evidence.

It does **not** close M001 or fully close M002 by itself. Both still wait for
the next ordinary stable B newer than 0.4.2:

- M002: first live Eggpack-produced five-target draft/publication evidence;
- M001 / Plan 106: real public 0.4.2 -> B updater transition.

No throwaway B is authorized.

## 12. Stop conditions

Stop and re-plan if:

- the current Eggup adapter no longer documents/implements a synchronous seam;
- another production StegoEggo subsystem genuinely requires `update_to` to
  remain async and a wider async ownership design is needed;
- the panic persists after removing the outer runtime, indicating a distinct
  Eggup/eggfetch defect;
- fixing the path requires changing fallback, trust, release-selection, or
  transaction policy.

## 13. Closure evidence

Record:

- implementation SHA(s);
- before/after updater call graph;
- direct Tokio dependency disposition;
- focused nested-runtime regression results;
- complete updater rehearsal result;
- fallback/error-policy regression matrix;
- standard/MSRV/release-drift CI results;
- dependency footprint delta if any;
- unresolved findings by severity/owner;
- confirmation that M001/M002 remain operationally gated only on the ordinary
  stable B event after this corrective.
