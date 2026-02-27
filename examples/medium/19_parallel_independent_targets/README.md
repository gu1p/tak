# medium/19_parallel_independent_targets

## Scenario Goal
Independent branches converging on one aggregate target.

Medium tier: combines multiple runtime and modeling features.

## What This Example Exercises
- fan-in DAG shape
- ordered completion before aggregate
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:aggregate`
3. `tak graph //:aggregate --format dot`
4. `tak run //:aggregate`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:aggregate`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/parallel.log`

## File Layout
- `TASKS.py`: project identity for this workspace (`module_spec(project_id=...)`).
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
