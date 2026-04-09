# medium/20_failure_no_retry_unmatched_exit

## Scenario Goal
Failure path when exit code does not match retry policy.

Medium tier: combines multiple runtime and modeling features.

## What This Example Exercises
- retry mismatch behavior
- expected terminal failure
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:failing`
3. `tak graph //:failing --format dot`
4. `tak run //:failing`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:failing`.
- `run`: expected success is `false`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `(none; failure scenario expected)`

## File Layout
- `TASKS.py`: project identity for this workspace (`module_spec(project_id=...)`).
- `TASKS.py`: root definitions used by the loader.
- Included `TASKS.py` files and scripts (if present): extra modules declared through `module_spec(includes=[...])` and step assets.
