# StegoEggo Planning and Agent-Handoff Process

Status: normative planning governance.

Adapted from CodeGG's planning convention to this repository. The keywords
MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, and MAY are normative.

## 1. Purpose

Two horizons MUST remain separate:

1. **Long-term planning** (`000`, `001`, `002`, this document):
   identity, invariants, terminology, sequencing, end-state acceptance.
2. **Interim planning** (`subsystems/`, `implementation/`, `closure/`,
   `archive/`, flat `001`–`107` history): bounded work against a
   particular repository baseline for handoff to coding agents.

Interim plans may discover evidence warranting a long-term change, but
they MUST NOT silently edit long-term direction to match the easiest
implementation. A corrective pass never justifies changing `000`.

## 2. Document classes

| Class | Location | Stability | Purpose |
|---|---|---|---|
| Canonical long-term | `plans/000-`, `001-`, `002-`, `003-` | Very stable; ADR or user direction to amend | What StegoEggo is becoming and must keep true |
| ADR | `plans/adrs/ADR-NNNN-*.md` | Immutable once accepted; supersede, don't rewrite | One durable decision across milestones |
| Subsystem roadmap | `plans/subsystems/<name>-roadmap.md` | Stable per workstream; may evolve with evidence | Ordered milestones for one workstream |
| Milestone plan | `plans/implementation/<subsystem>/NNN-*.md` | Operational; may be corrected | Bounded handoff (number local to subsystem) |
| Closure record | `plans/closure/<subsystem>/NNN-status.md` | Immutable except factual corrections | Evidence gate determining completion |
| Archive | `plans/archive/` | Historical only | Completed/superseded/abandoned interim docs |
| Flat history | `plans/001-*.md`–`plans/107-*.md` | Immutable except `-status.md` facts | Predecessor evidence, indexed by roadmaps |

## 3. Work classification

Every planned item MUST carry one primary class: **invariant** (must stay
true; needs guards/property tests), **capability** (user-visible; needs
end-to-end acceptance), **infrastructure** (internal machinery; MUST NOT
be presented as completed capability until a consumer path exists), or
**polish** (ergonomics/perf/docs; follows correctness closure).

## 4. Dependency model

Milestones declare dependencies as **hard** (cannot begin before
close), **interface** (may proceed against a stable written contract),
**soft** (parallel work, joint integration), or **operational** (landed
but release needs external evidence). Dependency-ready = all hard deps
closed + all interface deps contracted. `registry.md` names every
blocker and its owner.

## 5. Milestone sizing

One coherent pass: understand the boundary, implement, add focused tests,
run `./scripts/check.sh`, update docs, report residual risks. Too large =
several releasable boundaries, unrelated migrations, or several open
ADRs. Too small = symbol renames with no closure evidence (unless a
corrective unblock). Prefer vertical slices over horizontal refactors.

## 6. Agent handoff contract

Authority order: (1) `000`/`001`, (2) accepted ADRs, (3) subsystem
roadmap, (4) milestone plan, (5) repository evidence. On conflict,
preserve invariants, record the discrepancy, make the smallest coherent
adjustment; never invent architecture to finish a checklist. Agents MUST
inspect code before editing, preserve unrelated changes, keep examples
compiling, and report anything uncompleted. Stegoeggo specifics: byte
APIs when metadata matters, explicit seed + timestamp override for
reproducibility, `StegoError` in the `stego` surface, validated
`Redundancy`, no `unsafe`, no code comments unless asked, 4-space/100-col
rustfmt.

## 7. Corrective passes

A corrective pass is a NEW plan in the same subsystem (new local number),
referencing the original plan + closure, listing each unclosed finding,
explaining why verification missed it, and adding regression guards.
Repeated correctives signal the roadmap or sizing needs revision.

## 8. Registry requirements

`plans/registry.md` stays compact: active roadmaps, dependency-ready
plans, active/recent work, blocked work + blockers, latest closure. Link;
never duplicate requirements.

## 9. Required planning review

Before handoff: correct long-term refs; ADRs-or-out-of-scope for open
decisions; dependency readiness; bounded scope + non-goals; ownership +
invariants; migration/compat; failure/cancellation semantics; security
effects ( forgeable-without-key honesty, bounded parsing, no secret
logging); test + guard evidence; unambiguous closure criteria. If any is
unanswerable, the work is not ready.

## 10. Verification bar

Minimum `./scripts/check.sh` (fmt, clippy `-D warnings`,
no-default-features check, workspace tests). Conformance
(`verify_metadata_conformance.sh --strict`), docs-rs, MSRV package,
fuzz, deny, and semver-checks are specialist/manual — cite them only
when actually run. Closure MUST record exact commands + results (pass,
fail, skipped scope, counts); compilation alone never closes a
milestone, nor does infrastructure alone close a capability.

## 11. Anti-patterns (prohibited)

Transient TODOs in `000`–`002`; one roadmap mixing all subsystems;
unbounded product-goal handoffs; compilation-as-closure; per-subsystem
terminology drift; plans silently overriding ADRs; stale active plans
after material repo change; duplicated requirements without one source;
success-only evidence; polish before correctness; editing flat history
beyond `-status.md` facts; extending legacy builders instead of
`ProtectionRequest`.
