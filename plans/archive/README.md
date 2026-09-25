# Interim Planning Archive

Completed, superseded, rejected, or abandoned interim planning retained
for traceability. Not an active work queue — use `plans/registry.md`
and current roadmaps to find executable work.

## What belongs here

- completed milestone plans after closure;
- superseded roadmaps;
- corrective plans whose closure is complete;
- rejected interim proposals worth retaining.

## What does not belong here

- `000`–`003`, accepted ADRs, active roadmaps, ready/active plans,
  unresolved closure records, or flat `001`–`107` history (which stays
  in place as predecessor evidence).

## Archive layout

```text
archive/
    subsystems/<subsystem>-roadmap.md
    implementation/<subsystem>/NNN-short-title.md
    closure/<subsystem>/NNN-status.md
```

When archiving: update inbound links from `registry.md` and the current
roadmap; add a short archival note with final status and replacement;
prefer `git mv` to preserve history; never rewrite historical
conclusions; ensure active documents link to the replacement.
