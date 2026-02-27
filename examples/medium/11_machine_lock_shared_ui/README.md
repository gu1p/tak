# medium/11_machine_lock_shared_ui

## Scenario Goal
Daemon lock coordination for a shared machine resource.

Medium tier: combines multiple runtime and modeling features.

## What This Example Exercises
- machine-scope lock need
- lease pending/grant behavior
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:ui_test`
3. `tak graph //:ui_test --format dot`
4. `tak run //:ui_test`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:ui_test`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `true` (Required. Start daemon before running this example.)
- Required output files on successful run: `out/ui_lock.txt`

## File Layout
- `TASKS.py`: project identity for this workspace (`module_spec(project_id=...)`).
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
