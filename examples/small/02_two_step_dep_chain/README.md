# small/02_two_step_dep_chain

## Scenario Goal
Dependency-first execution in a short chain.

Small tier: focused behavior with minimal topology.

## What This Example Exercises
- explicit deps ordering
- downstream task execution
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //:test`
3. `tak graph //:test --format dot`
4. `tak run //:test`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:test`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/chain.log`

## File Layout
- `TASKS.py`: project identity for this workspace (`module_spec(project_id=...)`).
- `TASKS.py`: root definitions used by the loader.
- Included `TASKS.py` files and scripts (if present): extra modules declared through `module_spec(includes=[...])` and step assets.
