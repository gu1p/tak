# tak-exec Architecture

## Purpose

`tak-exec` is the runtime engine. It executes selected targets plus transitive dependencies in valid order, applying retries, step timeouts, and optional daemon lease coordination.

## Execution Model

1. Validate target set and run options.
2. Expand transitive dependency closure.
3. Topologically sort required tasks.
4. Resolve execution cascades and any fused container cascade candidates.
5. Execute tasks in order.
6. For each normal task attempt:
   - acquire the task lease when the task has `needs` and a lease socket is configured
   - for local host or local container placement, run steps after the lease is granted
   - for remote placement, submit to the remote node only after the lease is granted
   - release the lease after the attempt, including failure paths
   - evaluate retry policy/backoff
7. For each fused container cascade attempt:
   - build one synthetic fused task whose `needs` are merged from all fused members
   - resolve placement for that fused task
   - acquire one lease for the merged `needs`
   - run either the local fused member sequence or the remote fused submit
   - release the lease after the fused attempt, including failure paths

## Responsibilities

- dependency closure collection
- cycle-safe traversal guards
- execution cascade conflict detection
- fused container cascade planning
- step process spawning with env/cwd control
- live stdout/stderr relay for local host, local containerized, and remote containerized task attempts
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

- If a normal task attempt has `needs` and a socket is configured:
  - send acquire request until granted or terminal error
  - release the lease after each attempt, success or failure path
- Remote task attempts use the same client-side lease path:
  - acquire before remote submit
  - release after the remote attempt returns or the submit path fails
  - submitted `needs` are still sent to remote `takd` for status/reporting metadata
- Fused cascade attempts merge member `needs` into the synthetic fused task:
  - duplicate limiter references use the maximum slot request among fused members
  - acquire one lease before dispatching to local fused execution or remote fused submit
  - local fused execution must not acquire a second duplicate lease per member
- If no `needs` or no socket is configured:
  - run without daemon coordination

## Execution Cascade Semantics

- A task with cascaded execution applies its selected execution/session to its dependency closure.
- Two cascaded roots may share a dependency when they resolve to the same execution/session.
- If overlapping cascades resolve to different executions/sessions, the run fails before work starts.
- Containerized cascades can be fused into one per-run container for the dependency chain.
- Fused cascades report one result for the root and cover the member tasks in the scheduler.

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
- `plan_fused_cascades`
- `run_fused_cascade`
- `run_task_steps`
- `run_step`
- `acquire_task_lease` / `release_task_lease`

## Main Files

- `src/lib.rs`: execution orchestration and daemon integration.
