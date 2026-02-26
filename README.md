# Taskcraft

Taskcraft is a local task orchestrator for recursive monorepos. It loads distributed `TASKS.py` definitions, resolves them into one validated dependency graph, executes targets in dependency order, and optionally coordinates shared machine resources through `taskcraftd`.

## What Problem It Solves

- teams define tasks near code (`TASKS.py` in many folders)
- orchestration still works as one global graph
- resource contention (CPU, RAM, locks, queues) can be coordinated centrally
- retries/timeouts are standardized instead of copy-pasted in scripts

## Core Capabilities

- Recursive workspace loading with gitignore-aware discovery
- Strict label parsing (`//pkg:task`, `:task`)
- Topological execution with missing-dep/cycle validation
- Step execution for commands and scripts
- Retry policy with fixed or exponential-jitter backoff
- Optional daemon lease coordination for `needs`
- SQLite-backed daemon recovery/history

## Crate Map

- `crates/taskcraft-core`: canonical model types, labels, DAG planner
- `crates/taskcraft-loader`: `TASKS.py` discovery/evaluation/merge
- `crates/taskcraft-exec`: runtime executor and lease-client integration
- `crates/taskcraftd`: unix-socket daemon + lease manager + sqlite persistence
- `crates/taskcraft`: CLI surface, output contracts, dispatch

## CLI Quick Reference

- `taskcraft list`
  - answers: "what tasks exist?"
  - output: one fully-qualified label per line
- `taskcraft explain <label>`
  - answers: "what does this task depend on / contain?"
  - output: label, deps, steps, needs, timeout, retry attempts
- `taskcraft graph [label] --format dot`
  - answers: "what graph should be visualized?"
  - output: DOT graph
- `taskcraft web [label]`
  - answers: "show me this graph interactively in a browser"
  - output: local URL + embedded web graph server until `Ctrl+C`
- `taskcraft run <label...>`
  - answers: "did execution succeed and how many attempts were needed?"
  - output: one status line per executed task
- `taskcraft status`
  - answers: "what is daemon lease pressure right now?"
  - output: active leases, pending requests, limiter usage rows
- `taskcraft daemon start|status`
  - answers: "start daemon" / "is daemon responsive"

For full command contracts, see [`crates/taskcraft/ARCHITECTURE.md`](crates/taskcraft/ARCHITECTURE.md).

## Quickstart

1. Start daemon in a separate terminal:

```bash
taskcraft daemon start
```

2. In a workspace, inspect and run targets:

```bash
taskcraft list
taskcraft explain //apps/web:test_ui
taskcraft graph //apps/web:test_ui --format dot
taskcraft run //apps/web:test_ui
```

## Examples

- Full matrix in [`examples/catalog.toml`](examples/catalog.toml)
- Human index in [`examples/README.md`](examples/README.md)
- Contract test runs every entry: `crates/taskcraft/tests/examples_matrix_contract.rs`

## Quality Gates

```bash
make check
```

`make check` runs:

- formatting check
- clippy (warnings denied)
- workspace tests
- doctests for all crates
- docs policy contract (`doctest_contract`)

## Documentation Map

- System overview: [`ARCHITECTURE.md`](ARCHITECTURE.md)
- Core internals: [`crates/taskcraft-core/ARCHITECTURE.md`](crates/taskcraft-core/ARCHITECTURE.md)
- Loader internals: [`crates/taskcraft-loader/ARCHITECTURE.md`](crates/taskcraft-loader/ARCHITECTURE.md)
- Executor internals: [`crates/taskcraft-exec/ARCHITECTURE.md`](crates/taskcraft-exec/ARCHITECTURE.md)
- Daemon internals: [`crates/taskcraftd/ARCHITECTURE.md`](crates/taskcraftd/ARCHITECTURE.md)
- CLI contracts: [`crates/taskcraft/ARCHITECTURE.md`](crates/taskcraft/ARCHITECTURE.md)
