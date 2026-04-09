# medium/14_queue_priority_behavior

## Scenario Goal
Priority queue discipline usage contract.

Medium tier: combines multiple runtime and modeling features.

## What This Example Exercises
- queue priority values
- priority scheduling semantics
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:queued_priority`
3. `tak graph //:queued_priority --format dot`
4. `tak run //:queued_priority`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:queued_priority`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `true` (Required. Start daemon before running this example.)
- Required output files on successful run: `out/queue_priority.txt`

## File Layout
- `TASKS.py`: project identity for this workspace (`module_spec(project_id=...)`).
- `TASKS.py`: root definitions used by the loader.
- Included `TASKS.py` files and scripts (if present): extra modules declared through `module_spec(includes=[...])` and step assets.
