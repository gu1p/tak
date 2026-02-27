# small/09_retry_exp_jitter_match_exit

## Scenario Goal
Retry success path with exponential-jitter backoff configuration.

Small tier: focused behavior with minimal topology.

## What This Example Exercises
- exp-jitter strategy parsing
- retry-on-exit behavior
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:flaky_jitter`
3. `tak graph //:flaky_jitter --format dot`
4. `tak run //:flaky_jitter`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:flaky_jitter`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/retry_jitter.txt`

## File Layout
- `TASKS.py`: project identity for this workspace (`module_spec(project_id=...)`).
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
