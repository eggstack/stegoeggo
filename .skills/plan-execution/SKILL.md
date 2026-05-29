---
name: plan-execution
description: Use when executing tasks from plans/plan.md or any multi-wave parallel task plan. Triggers on tasks like "execute plan", "work through plan items", "run parallel tasks", or when working from a structured task list with worktrees.
---

# Plan-Based Task Execution

Workflow for executing multi-wave parallel tasks in the cloakrs codebase using git worktrees.

## Quick Reference

- Plans live in `plans/`
- Worktrees go in `/Users/davidbowman/projects/cloak-wt-taskN`
- Each task gets its own branch: `fix/taskN-description`
- Branch naming: `fix/` prefix for bug fixes, `feat/` for features

## Workflow

### 1. Read the plan completely

Understand all tasks, their dependencies, and which are parallelizable.

### 2. Create worktrees

```bash
# One per task, from the current HEAD
git worktree add /Users/davidbowman/projects/cloak-wt-taskN -b fix/taskN-description master
```

### 3. Launch parallel agents

Use the Task tool with `subagent_type: general` for each task. Each agent must:
- Work in its own worktree directory
- Read the relevant source files
- Implement the fix per the plan's steps
- Run `cargo test` and `cargo clippy --all-targets -- -D warnings`
- Commit changes with a descriptive message

### 4. Review worktree diffs

Before merging, review each worktree's changes:
```bash
cd /Users/davidbowman/projects/cloak-wt-taskN
git log --oneline -3
git diff HEAD~1 --stat
git diff HEAD~1 -- src/specific_file.rs
```

### 5. Merge to main

```bash
git merge fix/taskN-description --no-edit
```

Handle merge conflicts if they arise (unlikely for independent tasks).

### 6. Run full test suite on main

```bash
cargo test --all-features
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### 7. Update documentation

- Update AGENTS.md with new conventions/gotchas
- Update AGENTS.override.md with implementation notes
- Update the plan file to mark tasks completed
- Create/update skills in .skills/ if new patterns emerged

### 8. Clean up

```bash
git worktree remove /Users/davidbowman/projects/cloak-wt-taskN
```

## Gotchas

- **Worktree isolation**: Each agent MUST work in its own worktree. Overlapping writes cause agents to loop reverting each other.
- **Branch conflicts**: Don't create worktrees from the same branch if tasks touch the same files.
- **Test before merge**: Always run the full test suite on main after merging all worktrees.
- **Format after merge**: Merges may introduce formatting inconsistencies — run `cargo fmt` after all merges.
- **Plan file updates**: Only mark tasks complete after verifying the actual code changes, not just agent reports.

## Agent Prompt Template

When launching agents for plan tasks, use this template:

```
You are working on the cloakrs Rust codebase. Your task is to [description].

**Working directory**: `/Users/davidbowman/projects/cloak-wt-taskN`
**Branch**: `fix/taskN-description`

**Background**: [context from plan]

**Implementation Steps**: [from plan]

**Key details**: [from plan]

**Constraints**:
- Do not rewrite complex code from scratch
- Preserve the public API
- Follow existing code conventions (no comments in code unless asked, 4-space indent)
- Do not expand scope beyond this task
- Make small local changes

When done, report back with: what you changed, test results, and any issues encountered.
```
