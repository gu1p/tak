# large/21_recursive_enterprise_monorepo

## Scenario Goal
Enterprise-style app/platform dependency graph composed through explicit includes.

Large tier: explicit include topology and realistic multi-package flow.

## What This Example Exercises
- multi-package composition through root `includes=[...]`
- release target fan-in
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //apps/portal:release`
3. `tak graph //apps/portal:release --format dot`
4. `tak run //apps/portal:release`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//apps/portal:release`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/enterprise.log`

## File Layout
- `TASKS.py`: project identity for this workspace (`module_spec(project_id=...)`).
- `TASKS.py`: root definitions plus explicit `includes=[...]` used by the loader.
- Included package `TASKS.py` files and scripts (if present): task definitions and step assets.
