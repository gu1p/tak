# tak-exec Architecture

## Purpose

`tak-exec` is the runtime engine. It executes selected targets plus transitive dependencies in valid order, applying retries, step timeouts, and optional daemon lease coordination.

## Execution Model

1. Validate target set and run options.
2. Expand transitive dependency closure.
3. Topologically sort required tasks.
4. Execute tasks in order.
5. For each task attempt:
   - optionally acquire lease
   - run steps (cmd/script)
   - release lease
   - evaluate retry policy/backoff

## Responsibilities

- dependency closure collection
- cycle-safe traversal guards
- step process spawning with env/cwd control
- live stdout/stderr relay for local, remote, and containerized task attempts
- retry policy evaluation (`on_exit`, attempts, backoff)
- timeout enforcement per step
- daemon request/response handling for lease operations

## Input/Output Contracts

- Input
  - `WorkspaceSpec`
  - target labels
  - `RunOptions`
- Output
  - `RunSummary` keyed by `TaskLabel`
  - each result includes `success`, `attempts`, `exit_code`

## Lease Coordination Semantics

- If task has `needs` and socket is configured:
  - send acquire request until granted or terminal error
  - lease released after each attempt, success or failure path
- If no `needs` or no socket:
  - run locally without daemon coordination

## Failure Classes

- unknown target labels
- invalid run options (e.g. `jobs == 0`)
- process launch failures
- step non-zero exit
- timeout cancellation
- daemon transport/protocol errors
- lease release failures

## Main Functions

- `run_tasks`
- `run_single_task`
- `run_task_steps`
- `run_step`
- `acquire_task_lease` / `release_task_lease`

## Main Files

- `src/lib.rs`: execution orchestration and daemon integration.
