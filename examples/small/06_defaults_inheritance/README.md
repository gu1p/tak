# small/06_defaults_inheritance

## Scenario Goal
Application of module defaults into task-level runtime fields.

Small tier: focused behavior with minimal topology.

## What This Example Exercises
- default retry/queue/tags behavior
- task-local override fallback
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:apply_defaults`
3. `tak graph //:apply_defaults --format dot`
4. `tak run //:apply_defaults`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:apply_defaults`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/defaults.txt`

## File Layout
- `TASKS.py`: project identity for this workspace (`module_spec(project_id=...)`).
- `TASKS.py`: root definitions used by the loader.
- Included `TASKS.py` files and scripts (if present): extra modules declared through `module_spec(includes=[...])` and step assets.
