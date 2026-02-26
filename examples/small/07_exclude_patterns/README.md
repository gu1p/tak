# small/07_exclude_patterns

## Scenario Goal
Recursive discovery while excluding matched directories/files.

Small tier: focused behavior with minimal topology.

## What This Example Exercises
- exclude filtering
- loader file selection
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:main`
3. `tak graph //:main --format dot`
4. `tak run //:main`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:main`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/exclude.txt`

## File Layout
- `tak.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
