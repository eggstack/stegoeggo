# API and CLI Contract Roadmap

Status: closed

Long-term references:

- `plans/000-long-term-specification.md#2-canonical-api-invariants`
- `plans/000-long-term-specification.md#4-cli-invariants`
- `plans/001-terminology-and-domain-model.md#core-request-model`
- `plans/002-long-term-roadmap.md#phase-3--apicli-consolidation`

Related ADRs:

- `plans/adrs/ADR-0001-canonical-protection-request.md`
- `plans/adrs/ADR-0003-byte-vs-pixel-paths.md`

## 1. Purpose and ownership boundary

Owns the public Rust surface (`ProtectionRequest` convergence, auxiliary
APIs, legacy adapters, module decomposition) and the CLI contract
(commands, flags, exit codes, JSON output, docs). Owns `src/types/`,
`src/lib.rs` orchestration surface, `stegoeggo-cli/` routing. Does not
own carriers, containers, or release binaries.

## 2. Work classification

### Invariants

- New features on `ProtectionRequest` first; legacy translates only.
- CLI protection routes through
  `request::build_protection_request_with_explicit_options` + byte APIs.
- Exit codes 0–5 and `version` first-line contract.

### Capabilities

- `protect`/`inspect`/`verify`/`version`/`update`, `keygen`/`sign`/
  `verify-manifest` (signatures), conformance binary.

### Infrastructure

- Request resolution, plan executors, module facades.

### Polish

- Robustness/CLI polish, binary-size measurement, contract
  consolidation.

## 3. Non-goals

Carrier algorithms, container codecs, updater transport internals,
release publication.

## 4. Current state

Closed at 0.4.2. `STABILITY.md` freezes stable functions/types/commands;
deprecated adapters retained through 0.x. Evidence:
`tests/request_api.rs`, `tests/request_aux_convergence.rs`,
`tests/robustness.rs`, `tests/plan065_legacy_compat.rs`,
`examples/` compiling.

## 5. Target architecture

No change through 0.x; v1 removals deferred to Phase 5.

## 6. Dependency graph

Rights-metadata M1–M2 + stego-carrier M3 (interface contracts, stable)
→ API/CLI milestones (hard). All closed.

## 7. Milestones

- M1 robustness/CLI polish + default-policy/equivalence correctives —
  flat `012`, `047`. Class: polish/capability.
- M2 API/CLI consolidation + binary size — flat `042`, `043`.
  Class: capability/polish.
- M3 post-corrective evidence closures — flat `050`, `051`.
  Class: polish.
- M4 pre-v1 consolidation + request/verification convergence + module
  decomposition — flat `061`, `081`, `082`, `083`, `084`.
  Class: infrastructure/invariant.
- M5 CLI dependency consolidation + roadmap/fuzz correctives — flat
  `086`–`089` (interface to release-distribution). Class: polish.
- M6 command-surface simplification — flat `098`. Class: capability.

## 8. Cross-cutting requirements

Compat: 0.x aliases (root positional syntax, `--verify`) retained;
removal tracked in `DEPRECATIONS.md`. Docs: `docs/cli-usage.md`,
`docs/rust-api.md`, `docs/migration-v0.3.md`,
`architecture/cli.md`. MSRV 1.89; `forbid(unsafe_code)`; examples keep
compiling.

## 9. Verification strategy

Request/compat/robustness suites + `./scripts/check.sh`; CLI rehearsal
scripts for installer/updater-adjacent claims live in
release-distribution.

## 10. Risks and decision points

None open. `has_notice()`-for-any-DMI and `Option<bool>` injection
semantics are documented gotchas.

## 11. Completion definition

Met: converged canonical surface with frozen stability promises and a
documented compatibility boundary.

## 12. Milestone status table

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M1 polish/policy | closed | flat `012`/`047` | `047-status.md` | — |
| M2 consolidation/size | closed | flat `042`/`043` | `042-status.md`, `043-status.md` | — |
| M3 evidence closure | closed | flat `050`/`051` | `050-status.md`, `051-status.md` | — |
| M4 pre-v1 convergence | closed | flat `081`/`082`/`083`/`084` | `081-status.md`–`084-status.md` (as present) | — |
| M5 dep consolidation | closed | flat `086`–`089` | `086-status.md`–`089-status.md` (as present) | — |
| M6 command surface | closed | flat `098` | `098-status.md` | — |
