# medium/17_scope_user_project_worktree_mix

## Scenario Goal
Scope key derivation across user/project/worktree contexts.

Medium tier: combines multiple runtime and modeling features.

## What This Example Exercises
- scope-key resolution
- multi-scope limiter references
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:scoped_task`
3. `tak graph //:scoped_task --format dot`
4. `tak run //:scoped_task`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:scoped_task`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `true` (Required. Start daemon before running this example.)
- Required output files on successful run: `out/scopes.txt`

## File Layout
- `tak.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
