# small/06_defaults_inheritance

## Scenario Goal
Application of module defaults into task-level runtime fields.

Small tier: focused behavior with minimal topology.

## What This Example Exercises
- default retry/queue/tags behavior
- task-local override fallback
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `taskcraft list`
2. `taskcraft explain //:apply_defaults`
3. `taskcraft graph //:apply_defaults --format dot`
4. `taskcraft run //:apply_defaults`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:apply_defaults`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/defaults.txt`

## File Layout
- `taskcraft.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
