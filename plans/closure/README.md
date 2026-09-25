# Closure and Verification Records

Evidence-based completion records. A closure record is the gate that
determines whether a milestone is complete — a "closed" commit message
is not evidence.

## Layout and naming

```text
closure/<subsystem>/NNN-status.md
```

Same milestone number as the source implementation plan.

## Required template

```markdown
# <Subsystem> Milestone NNN — Closure Status

Status: closed | conditionally closed | corrective pass required | blocked
Source implementation plan: `plans/implementation/<subsystem>/NNN-...md`
Source subsystem roadmap: `plans/subsystems/<subsystem>-roadmap.md#...`
Repository baseline reviewed: `<SHA>`
Implementation commits: `<SHA> — summary`

## 1. Executive finding
## 2. Requirement-to-evidence matrix
## 3. Production implementation evidence
## 4. Verification executed (exact commands + results)
## 5. Invariant review
## 6. Failure and recovery review
## 7. Migration and compatibility review
## 8. Security review
## 9. Documentation and operations
## 10. Unresolved findings (critical/high/medium/low)
## 11. Roadmap disposition
## 12. Registry updates
```

## Closure rules

MUST NOT mark closed when only compilation/formatting was verified,
required tests didn't run, a capability has only infrastructure, a
security/migration requirement is unimplemented, a known high-severity
defect remains, or closure rests on unrecorded assumptions.
Conditionally closed needs the exact outstanding evidence named.
Corrective follow-up keeps this record immutable and opens a new
implementation plan.
