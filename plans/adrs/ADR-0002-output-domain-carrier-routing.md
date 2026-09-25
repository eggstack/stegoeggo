# ADR-0002: Output-Domain Carrier Routing with JPEG Byte-Only Fast Path

Status: accepted

Date: 2026-09-25

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#3-execution-invariants`
- `plans/001-terminology-and-domain-model.md#marker-channel`

Affected subsystem roadmaps:

- `plans/subsystems/stego-carrier-roadmap.md`
- `plans/subsystems/container-correctness-roadmap.md`

## Context

Carrier selection could plausibly follow input format, output format, or
negotiated transcode. Mixed routing caused transient-DCT detours and
re-encode artifacts. Predecessor evidence: flat plans `040`, `061`,
`077`, `078`.

## Decision drivers

- JPEG→JPEG must avoid lossy pixel decode/encode cycles.
- JPEG→PNG/WebP must never pass through a transient DCT step.
- One router owns the decision.

## Considered options

### Option A — Output-domain routing, single router

`output_format == JPEG ? DCT : LSB`; `execute_full_marker_and_metadata()`
in `src/pipeline.rs` is the sole current-carrier router. Benefits:
predictable, one audit point. Costs: format-conversion callers must
accept raster LSB. Failure mode: router bypass — mitigated by review
rule (raster helper must never branch on input format).

### Option B — Input-domain routing with transcode hints

Benefits: preserves input carrier investment. Costs: lossy double
compression on JPEG→JPEG, transient DCT on JPEG→PNG. Rejected.

## Decision

Option A. JPEG→JPEG uses the carrier's byte-only DCT path; JPEG→PNG/WebP
is one pixel decode plus LSB.

## Consequences

### Positive

- No re-compression artifacts on the JPEG fast path; APP/COM/unknown
  segments survive verbatim.
- `EmbedPath` reflects the operation actually executed.

### Negative

- Callers requesting JPEG markers but PNG output silently get LSB
  semantics; must be documented, not hidden.

### Neutral or deferred

- None.

## Compatibility and migration

No wire change; V3 payloads identical across carriers. Progressive JPEGs
fall back to seed-in-Q-tables (documented limitation).

## Security and reliability implications

Fewer lossy cycles preserve marker integrity; capacity units are
eligible AC coefficients (`|coef| >= 2`), checked before mutation.

## Verification

`tests/output_domain_routing.rs`, `tests/jpeg_container_preservation.rs`,
`tests/cross_format_semantics.rs`, plus `./scripts/check.sh`.

## Supersession

None.
