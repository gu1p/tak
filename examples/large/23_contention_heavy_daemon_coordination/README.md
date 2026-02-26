# large/23_contention_heavy_daemon_coordination

## Scenario Goal
High-contention daemon-managed coordination scenario.

Large tier: recursive topology and realistic multi-package flow.

## What This Example Exercises
- multiple contention points
- lease queue pressure
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `taskcraft list`
2. `taskcraft explain //:orchestrate`
3. `taskcraft graph //:orchestrate --format dot`
4. `taskcraft run //:orchestrate`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:orchestrate`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `true` (Required. Start daemon before running this example.)
- Required output files on successful run: `out/contention.log`

## File Layout
- `taskcraft.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
