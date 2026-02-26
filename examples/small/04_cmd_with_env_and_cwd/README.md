# small/04_cmd_with_env_and_cwd

## Scenario Goal
Command step behavior with explicit environment variables and working directory.

Small tier: focused behavior with minimal topology.

## What This Example Exercises
- `env` injection
- `cwd` path resolution
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:env_cmd`
3. `tak graph //:env_cmd --format dot`
4. `tak run //:env_cmd`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:env_cmd`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/marker.txt`

## File Layout
- `tak.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
