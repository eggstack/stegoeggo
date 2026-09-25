# Architecture Decision Records

Durable decisions affecting StegoEggo architecture across milestones or
subsystems. Numbers are monotonically increasing and never reused.

## Status lifecycle

```text
proposed -> accepted -> deprecated or superseded
         `-> rejected
```

Accepted ADRs are historical records. Never rewrite one to make a later
decision appear original; create a new ADR and mark the old one
superseded.

## When an ADR is required

- canonical identity or ownership boundary changes;
- new container/metadata/carrier protocol or durable external standard;
- authentication/authorization semantics changes;
- public compatibility contract established;
- a long-term non-goal materially changes.

Unnecessary for local refactors, internal naming, implementation-specific
data structures, or reversible optimizations preserving contracts.

## Template

See any `ADR-*.md` in this directory: Status, Date, Decision owners,
Related specification sections, Affected subsystem roadmaps, Context,
Decision drivers, Considered options (benefits/costs/failure modes),
Decision, Consequences (positive/negative/neutral), Compatibility and
migration, Security and reliability implications, Verification,
Supersession.
