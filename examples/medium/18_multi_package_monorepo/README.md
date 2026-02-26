# medium/18_multi_package_monorepo

## Scenario Goal
Recursive package graph assembly in a medium monorepo.

Medium tier: combines multiple runtime and modeling features.

## What This Example Exercises
- nested TASKS.py merge
- cross-package dependencies
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `taskcraft list`
2. `taskcraft explain //apps/web:all`
3. `taskcraft graph //apps/web:all --format dot`
4. `taskcraft run //apps/web:all`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//apps/web:all`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/monorepo.log`

## File Layout
- `taskcraft.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
