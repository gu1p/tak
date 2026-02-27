# small/08_retry_fixed_fail_once

## Scenario Goal
Deterministic fail-once then retry success using fixed backoff.

Small tier: focused behavior with minimal topology.

## What This Example Exercises
- retry `attempts` contract
- exit-code matching for retry
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:flaky_fixed`
3. `tak graph //:flaky_fixed --format dot`
4. `tak run //:flaky_fixed`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:flaky_fixed`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/retry_fixed.txt`

## File Layout
- `TASKS.py`: project identity for this workspace (`module_spec(project_id=...)`).
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
