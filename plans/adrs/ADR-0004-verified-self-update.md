# ADR-0004: Verified Self-Update with Crates.io Authority

Status: accepted

Date: 2026-09-25

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#4-cli-invariants`
- `plans/000-long-term-specification.md#5-release-invariants`

Affected subsystem roadmaps:

- `plans/subsystems/release-distribution-roadmap.md`

## Context

The updater must resolve "latest" from a trusted authority, fetch a
matching binary, and never leave a broken executable behind. Predecessor
evidence: flat plans `100`, `103`, `104`, `105`.

## Decision drivers

- Stable-only authority (no prereleases, no yanked).
- Binary provenance: exact-asset match + checksum + identity + version.
- Current executable untouched on any staging/validation failure.
- Offline `version`; no `sudo`; bounded network behavior.

## Considered options

### Option A — Crates.io stable authority, GitHub asset fulfillment

Query crates.io `stegoeggo-cli` stable version via embedded eggfetch,
check destination replaceability (skip when already current), download
the exact target asset + `.sha256` sidecar, verify all three gates,
then self-replace. Cargo fallback only for unsupported targets or
exact-asset HTTP 404. Benefits: registry-anchored identity, reproducible
policy matrix. Costs: two-service dependency. Failure mode: service
outage — fails closed, executable preserved.

### Option B — GitHub-only latest resolution

Benefits: one service. Costs: release-draft/tag ambiguity, weaker
stable/yanked semantics. Rejected.

## Decision

Option A with the updater invariants in AGENTS.md (bounded eggfetch
transport, explicit proxy handling, HTTPS-downgrade denial, argument-array
subprocesses, no `sudo`).

## Consequences

### Positive

- Already-current installs report current without destination preflight.
- Checksum/identity/network failures are fatal, never Cargo-masked.

### Negative

- Maintains fixture rehearsal scripts (`test-release-updater.sh`).

### Neutral or deferred

- Real A→B public proof waits for release B (flat Plan 106, blocked).

## Compatibility and migration

No manifest or CLI flag change; bootstrap installers verify the same
sidecar/identity gates.

## Security and reliability implications

Downgrade denied; oversized registry/sidecar/executable rejected; proxy
credentials never emitted; staged failures preserve the executable.

## Verification

`cargo test -p stegoeggo-cli --all-features`,
`./scripts/test-release-updater.sh`, real-transition evidence per Plan
106 at release-B time.

## Supersession

None.
