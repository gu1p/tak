# medium/16_process_cap_guard

## Scenario Goal
Process-cap limiter behavior for guarded task execution.

Medium tier: combines multiple runtime and modeling features.

## What This Example Exercises
- process cap declarations
- guarded runtime acquisition
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:process_guarded`
3. `tak graph //:process_guarded --format dot`
4. `tak run //:process_guarded`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:process_guarded`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `true` (Required. Start daemon before running this example.)
- Required output files on successful run: `out/process_cap.txt`

## File Layout
- `TASKS.py`: project identity for this workspace (`module_spec(project_id=...)`).
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
