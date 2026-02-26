# small/09_retry_exp_jitter_match_exit

## Scenario Goal
Retry success path with exponential-jitter backoff configuration.

Small tier: focused behavior with minimal topology.

## What This Example Exercises
- exp-jitter strategy parsing
- retry-on-exit behavior
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `taskcraft list`
2. `taskcraft explain //:flaky_jitter`
3. `taskcraft graph //:flaky_jitter --format dot`
4. `taskcraft run //:flaky_jitter`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:flaky_jitter`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/retry_jitter.txt`

## File Layout
- `taskcraft.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
