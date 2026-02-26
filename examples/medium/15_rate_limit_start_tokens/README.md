# medium/15_rate_limit_start_tokens

## Scenario Goal
Rate limiter declaration and runtime need usage.

Medium tier: combines multiple runtime and modeling features.

## What This Example Exercises
- rate-limit limiter definition
- token-based startup control
- command trio contract: `list`, `explain`, `graph`, `run`

## Runbook
1. `taskcraft list`
2. `taskcraft explain //:rate_limited`
3. `taskcraft graph //:rate_limited --format dot`
4. `taskcraft run //:rate_limited`

## Expected Command Answers
- `list`: includes fully-qualified labels relevant to this scenario.
- `explain`: returns task metadata fields (`label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`).
- `graph --format dot`: prints DOT dependency edges for `//:rate_limited`.
- `run`: expected success is `true`.

## Expected Artifacts
- Required daemon: `true` (Required. Start daemon before running this example.)
- Required output files on successful run: `out/rate_limit.txt`

## File Layout
- `taskcraft.toml`: project identity for this workspace.
- `TASKS.py`: root definitions used by loader.
- Nested `TASKS.py` and scripts (if present): recursive modules and step assets.
