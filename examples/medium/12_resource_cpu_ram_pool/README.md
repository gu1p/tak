# medium/12_resource_cpu_ram_pool

## Scenario Goal
Resource slot accounting with CPU and RAM needs.

Medium tier: combines multiple runtime and modeling features.

## What This Example Exercises
- multi-resource acquisition
- capacity bookkeeping
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:heavy`
3. `tak graph //:heavy --format dot`
4. `tak run //:heavy`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:heavy`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `true` (Required. Start daemon before running this example.)
- Required output files on successful run: `out/resources.txt`

## File Layout
- `TASKS.py`: project identity for this workspace (`module_spec(project_id=...)`).
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
