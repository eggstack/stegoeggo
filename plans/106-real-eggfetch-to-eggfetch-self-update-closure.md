# Plan 106: Real Eggfetch-to-Eggfetch Self-Update Closure

## Goal

Close the final release-dependent validation gap for the native updater by
performing one real public stable-release transition:

```text
eggfetch-enabled release A
    -> stegoeggo update
    -> eggfetch-enabled release B
```

Both A and B must be publicly published stable StegoEggo CLI releases whose
binaries contain the native `eggfetch-core` updater from Plan 103.

This plan should normally execute during the first ordinary stable release
after Plan 105. Do not publish a throwaway version solely to satisfy this test.

## Preconditions

Plan 105 must be complete.

There must be:

- a stable release A whose CLI binary contains the native eggfetch updater;
- a newer stable release B whose CLI binary also contains the native eggfetch
  updater;
- matching crates.io `stegoeggo-cli` B publication;
- matching GitHub Release `vB`;
- complete B release asset matrix and checksum sidecars.

If B does not yet exist, leave this plan planned/pending rather than simulating
the public transition.

## Scope

In scope:

- install or stage release A in an isolated environment;
- verify A's identity;
- run A's real `stegoeggo update` against public crates.io/GitHub endpoints;
- verify B is resolved as the latest stable version;
- verify B's target-specific release asset and checksum are used;
- verify replacement succeeds;
- verify post-update binary reports exactly B;
- verify a second `stegoeggo update` is a clean no-op;
- exercise representative failure cases against deterministic fixtures if any
  changed since Plan 105;
- record exact evidence in `plans/106-status.md`;
- reconcile release/updater documentation only if observed behavior differs
  from the documented contract.

Out of scope:

- changing updater architecture;
- changing version authority;
- changing fallback policy;
- changing asset naming;
- changing supported targets;
- publishing a special test-only stable version;
- using prereleases as a substitute for the stable crates.io authority.

## Workstream A: Identify Releases A and B

Record:

```text
A = first/older stable release containing native eggfetch updater
B = newer stable release containing native eggfetch updater
```

Verify both source tags include:

- `eggfetch-core` updater dependency;
- Rust 1.89 or later declared MSRV;
- native updater code in `stegoeggo-cli/src/update.rs`;
- the expected five-target release contract.

Confirm:

```bash
cargo info stegoeggo-cli@A
cargo info stegoeggo-cli@B
```

or equivalent crates.io API evidence.

Do not rely only on Git tags if the CLI crate was not published.

## Workstream B: Isolated A Installation

Install release A into a disposable path/environment.

Preferred approaches:

- pinned public bootstrap installer;
- direct verified release asset;
- isolated temporary HOME/PATH.

Do not overwrite the maintainer's primary installed binary.

Verify:

```bash
stegoeggo version
```

Expected:

```text
stegoeggo A
```

Record:

- host OS/architecture;
- asset name;
- installation method;
- SHA-256 verification result;
- executable path.

## Workstream C: Real Public A -> B Update

With release A active and public B already stable:

```bash
stegoeggo update
```

Do not override:

- `STEGOEGGO_CRATES_API_URL`;
- `STEGOEGGO_RELEASES_URL`.

The command must use the real production endpoints.

Expected sequence:

1. A queries crates.io through embedded eggfetch;
2. B is selected as newest stable non-yanked version;
3. A checks its destination replaceability before downloading B's release
   asset;
4. A selects the exact current host target;
5. A downloads B's exact GitHub Release executable;
6. A downloads B's exact checksum sidecar;
7. A verifies SHA-256;
8. A validates candidate identity and exact B version;
9. A self-replaces;
10. resulting binary reports B.

Capture stdout/stderr and exit status without exposing proxy credentials or
other secrets.

## Workstream D: Verify Updated Binary

Immediately verify:

```bash
stegoeggo version
stegoeggo --help
```

Expected version:

```text
stegoeggo B
```

Run a small functional smoke:

```bash
stegoeggo protect <fixture> --output <temp-output> --rights-policy prohibited-ai-ml-training --preset legal-notice
stegoeggo inspect <temp-output>
stegoeggo verify <temp-output>
```

This confirms the replacement produced a normal B CLI rather than only a
version-printing executable.

## Workstream E: Already-Current Second Run

Run:

```bash
stegoeggo update
```

again.

Expected:

- exit 0;
- B reported as current/latest;
- no release asset download;
- no self-replacement;
- no Cargo fallback.

This checks that the new B binary's own eggfetch updater is healthy after
replacement.

## Workstream F: Curl Independence

Where practical, run the A -> B transition with external `curl` absent from
PATH or otherwise unavailable to the installed CLI.

Requirements:

- the installed CLI update succeeds without curl;
- do not confuse this with bootstrap installation, which may still use curl;
- Cargo may remain present, but must not be invoked on the supported target
  when the exact B asset exists.

If PATH manipulation would hide required system tools for the test harness,
use a controlled environment that proves curl absence without destabilizing
the OS.

## Workstream G: Target Coverage

One real A -> B transition on a supported target is the minimum closeout gate.

Preferred primary target:

- a platform where self-replacement behavior is representative and safely
  isolated.

Additionally, if practical, run the transition on Windows because Windows has
different running-executable replacement semantics.

If native Windows A -> B cannot be executed during the release window, require
at least:

- Plan 105 native Windows B build/smoke evidence;
- existing deterministic/self-replace Windows-compatible code review;
- explicit status note that real Windows replacement remains observational
  evidence rather than a release blocker unless Windows behavior actually
  regresses.

Do not falsely claim a Windows A -> B run that did not happen.

## Workstream H: Failure-Mode Regression

Plans 103-105 already cover deterministic failure cases. Re-run the focused
updater rehearsal at B:

```bash
cargo test -p stegoeggo-cli --all-features
./scripts/test-release-updater.sh
```

Ensure these still cover:

- exact asset 404 may fall back to Cargo;
- sidecar 404 is fatal;
- 5xx is fatal;
- transport/TLS/timeout/proxy failure is fatal;
- checksum mismatch leaves current executable unchanged;
- wrong candidate identity/version leaves current executable unchanged;
- oversized registry/sidecar/executable is rejected;
- HTTPS downgrade is denied;
- proxy credentials are not emitted;
- unsupported target is the only non-404 binary-source fallback case.

No new fixture architecture is required if existing coverage still exercises
the production code path.

## Workstream I: Evidence and Documentation Closeout

Populate `plans/106-status.md` with:

- A version and tag;
- B version and tag;
- A and B source commit SHAs;
- test host/target;
- A installation method;
- A pre-update version output;
- updater output/exit result;
- B post-update version output;
- B functional smoke result;
- second update no-op result;
- whether curl was absent;
- whether Cargo fallback was observed (expected: no);
- Windows real-transition result if performed;
- deterministic regression results;
- any observed documentation discrepancy.

Update user/maintainer documentation only if the real transition contradicts a
current claim.

## Acceptance Criteria

Plan 106 is complete when:

- two stable public releases A < B both contain the native eggfetch updater;
- an isolated installed A binary successfully updates itself to B through the
  real public crates.io/GitHub path;
- B's exact checksum and candidate identity/version are validated before
  replacement;
- the resulting executable reports exactly B;
- normal protect/inspect/verify functionality works after replacement;
- a second update run reports already current and is non-destructive;
- the supported-target transition does not invoke Cargo fallback;
- updater operation does not depend on external curl;
- focused deterministic updater regression tests still pass;
- `plans/106-status.md` records reproducible evidence.

## Failure Policy

A failed real A -> B transition is a release/updater defect until explained.

Do not:

- bypass checksum validation;
- manually replace A with B and call that updater success;
- use fixture URLs as evidence for this plan's public transition;
- treat a curl-based pre-Plan-103 updater as release A;
- enable Cargo fallback for generic network failures;
- force-move tags or overwrite published crates;
- publish a meaningless stable version solely to make A -> B available.

If the failure requires an updater code change, write a new numbered
corrective plan and keep Plan 106 open until the corrected behavior is proven
on a subsequent real stable transition.

## Ordering

Plan 106 follows Plan 105 and executes at the first normal stable release after
Plan 105 that provides a newer eggfetch-enabled CLI version.
