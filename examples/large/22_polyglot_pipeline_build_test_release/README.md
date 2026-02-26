# large/22_polyglot_pipeline_build_test_release

## Scenario Goal
Polyglot release orchestration across language service packages.

Large tier: recursive topology and realistic multi-package flow.

## What This Example Exercises
- cross-language dependencies
- release pipeline sequencing
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `tak list`
2. `tak explain //services/python:release`
3. `tak graph //services/python:release --format dot`
4. `tak run //services/python:release`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//services/python:release`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `false` (Not required for this scenario.)
- Required output files on successful run: `out/polyglot_release.txt`

## File Layout
- `tak.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
