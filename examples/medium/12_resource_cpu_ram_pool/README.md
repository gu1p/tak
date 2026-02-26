# medium/12_resource_cpu_ram_pool

## Scenario Goal
Resource slot accounting with CPU and RAM needs.

Medium tier: combines multiple runtime and modeling features.

## What This Example Exercises
- multi-resource acquisition
- capacity bookkeeping
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `taskcraft list`
2. `taskcraft explain //:heavy`
3. `taskcraft graph //:heavy --format dot`
4. `taskcraft run //:heavy`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:heavy`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `true` (Required. Start daemon before running this example.)
- Required output files on successful run: `out/resources.txt`

## File Layout
- `taskcraft.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
