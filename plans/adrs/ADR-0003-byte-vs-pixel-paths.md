# ADR-0003: Byte Paths Carry Metadata, Pixel Paths Are Stego-Only

Status: accepted

Date: 2026-09-25

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#3-execution-invariants`
- `plans/001-terminology-and-domain-model.md#core-request-model`

Affected subsystem roadmaps:

- `plans/subsystems/api-cli-contract-roadmap.md`
- `plans/subsystems/rights-metadata-roadmap.md`

## Context

`Protector::apply()` returns `Cow::Borrowed` unchanged for metadata-only
resolves, and container metadata (PNG tEXt, JPEG COM/XMP, WebP XMP) does
not survive pixel round-trips. Callers using `process_image` while
expecting metadata silently got stego-only output. Predecessor evidence:
flat plans `022`, `042`, `082`.

## Decision drivers

- Make the footgun structurally visible instead of documented-only.
- Keep a zero-allocation pixel path for pixel-only consumers.

## Considered options

### Option A — Split contract (chosen)

Byte APIs (`process_request_bytes`, `process_image_bytes`) are the only
metadata-carrying paths; `DynamicImage` in/out APIs embed stego only.
Benefits: no silent metadata loss, no container re-synthesis cost on
pixel paths. Costs: two-path mental model. Failure mode: caller picks
the wrong path — mitigated by AGENTS.md top-footgun note and CLI routing
exclusively through byte APIs.

### Option B — Re-synthesize containers on pixel paths

Benefits: uniform semantics. Costs: format guessing, allocation, false
container fidelity. Rejected.

## Decision

Option A. All CLI protection routes through byte processing APIs.

## Consequences

### Positive

- Metadata expectations are honest; pixel path stays cheap.

### Negative

- New users must learn the split; guides must lead with byte APIs.

### Neutral or deferred

- None.

## Compatibility and migration

Pixel APIs keep signatures and stego behavior; `docs/rust-api.md` and
`architecture/pipeline.md` document the split.

## Security and reliability implications

Legal-evidence metadata cannot be silently dropped by a pixel round-trip
when callers follow the contract; nothing cryptographic either way.

## Verification

`tests/preservation.rs`, `tests/metadata_only_idempotence.rs`,
`tests/request_api.rs`, plus `./scripts/check.sh`.

## Supersession

None.
