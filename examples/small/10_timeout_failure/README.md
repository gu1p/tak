# small/10_timeout_failure

## Scenario Goal
Controlled timeout failure path.

Small tier: focused behavior with minimal topology.

## What This Example Exercises
- step timeout enforcement
- expected non-zero command outcome
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:slow_timeout`
3. `tak graph //:slow_timeout --format dot`
4. `tak run //:slow_timeout`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:slow_timeout`.
- `run`: expected success is `false`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `(none; failure scenario expected)`

## File Layout
- `tak.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
