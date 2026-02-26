# small/05_script_step_with_interpreter

## Scenario Goal
Script step invocation with explicit interpreter selection.

Small tier: focused behavior with minimal topology.

## What This Example Exercises
- `script(...)` execution
- interpreter override
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:script_gen`
3. `tak graph //:script_gen --format dot`
4. `tak run //:script_gen`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:script_gen`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/script.txt`

## File Layout
- `tak.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
