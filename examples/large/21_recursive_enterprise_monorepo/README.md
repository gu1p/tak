# large/21_recursive_enterprise_monorepo

## Scenario Goal
Enterprise-style recursive app/platform dependency graph.

Large tier: recursive topology and realistic multi-package flow.

## What This Example Exercises
- multi-level package traversal
- release target fan-in
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `taskcraft list`
2. `taskcraft explain //apps/portal:release`
3. `taskcraft graph //apps/portal:release --format dot`
4. `taskcraft run //apps/portal:release`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//apps/portal:release`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/enterprise.log`

## File Layout
- `taskcraft.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
