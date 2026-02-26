# large/24_full_feature_matrix_end_to_end

## Scenario Goal
Combined end-to-end scenario using limiters, queues, retries, scripts, and recursive packages.

Large tier: recursive topology and realistic multi-package flow.

## What This Example Exercises
- full-stack feature composition
- final release artifact validation
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //apps/qa:release`
3. `tak graph //apps/qa:release --format dot`
4. `tak run //apps/qa:release`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//apps/qa:release`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `true` (Required. Start daemon before running this example.)
- Required output files on successful run: `out/full_matrix_release.txt`

## File Layout
- `tak.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
