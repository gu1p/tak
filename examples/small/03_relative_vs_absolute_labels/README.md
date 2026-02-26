# small/03_relative_vs_absolute_labels

## Scenario Goal
Cross-package dependency resolution with relative and absolute labels.

Small tier: focused behavior with minimal topology.

## What This Example Exercises
- `:task` resolution
- `//pkg:task` resolution
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `taskcraft list`
2. `taskcraft explain //apps/web:test`
3. `taskcraft graph //apps/web:test --format dot`
4. `taskcraft run //apps/web:test`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//apps/web:test`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/labels.log`

## File Layout
- `taskcraft.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
