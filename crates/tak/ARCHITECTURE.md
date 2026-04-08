# tak CLI Architecture

## Purpose

`tak` is the user-facing interface over three subsystems:

- `tak-loader` for workspace/task graph discovery
- `tak-exec` for task execution
- `takd` for optional standalone remote execution agents

The CLI is query + action oriented: each command answers one operational question and prints a stable text response contract.

## Runtime Shape

- `src/main.rs`: process entrypoint, delegates to library runtime.
- `src/lib.rs`: clap parsing, command dispatch, and output formatting.

High-level flow:

1. Parse command with clap.
2. For workspace commands, load `WorkspaceSpec` from current working directory.
3. Dispatch to loader/executor/remote-inventory APIs.
4. Print line-oriented response to stdout.
5. Return non-zero exit on any `Result::Err`.

## Command Answer Matrix

| Command | Primary question answered | Backend calls | Output contract |
|---|---|---|---|
| `tak list` | "What tasks exist in this workspace?" | `load_workspace_from_cwd()` | One fully-qualified label per line, e.g. `//apps/web:test_ui`. |
| `tak explain <label>` | "What is this task composed of?" | workspace load + label parse + task lookup | Structured text fields: `label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`. |
| `tak graph [label] --format dot` | "What dependency graph should I visualize?" | workspace load + optional label parse | DOT graph text (`digraph tak { ... }`). |
| `tak web [label]` | "Show this graph interactively in browser" | workspace load + optional label parse + embedded local server | Prints local URL, serves embedded HTML/CSS/JS UI, runs until `Ctrl+C`. |
| `tak run <label...> [-j N] [--keep-going]` | "Execute these targets with dependencies" | workspace load + label parsing + `run_tasks(...)` | One result line per executed label: `<label>: ok|failed (attempts=X, exit_code=Y|none)`. |
| `tak status` | "Is live coordination status available here?" | none in the current client-only build | Returns an unsupported error. |
| `tak remote add <token>` | "Add a remote execution agent" | token decode + `/v1/node/info` probe (bounded retry for Tor onion remotes) + config write | `added remote <node_id>`. |
| `tak remote list` | "Which remote execution agents are configured?" | config read | One configured agent per line. |

## Output Details Per Command

### `list`

- Success output: newline-separated labels only.
- Typical use: scripting (`tak list | rg ...`).

### `explain`

- Success output shape:
  - `label: <label>`
  - `deps: (none)` or dependency list prefixed with `  - `
  - `steps: <count>`
  - `needs: <count>`
  - `timeout_s: <seconds|none>`
  - `retry_attempts: <count>`

### `graph`

- Only `--format dot` is supported.
- Output is valid DOT suitable for Graphviz tooling.

### `run`

- Requires at least one label.
- Delegates retry, timeout, and lease behavior to `tak-exec`.
- Per-task status is printed after execution summary is available.

### `web`

- Uses embedded frontend assets and vendored graph library files.
- Binds `127.0.0.1` on a random available port.
- Prints URL and serves graph UI until interrupted.
- Auto-opens browser only in production-style runtime (`!debug_assertions`) and when `TAK_NO_BROWSER_OPEN` is not set.
- If open fails, command keeps serving and prints the manual URL.

### `status`

- Returns an unsupported error in the current client-only build.
- Exists so the CLI surface can reserve the status verb until live coordination status is restored.

## Error and Exit Semantics

All commands fail fast on errors and return non-zero exit status.

Common failure classes:

- Workspace load/parse errors (`TASKS.py`, label resolution).
- Invalid CLI input (unsupported graph format, missing run labels, bad label syntax).
- Execution failures (`run` task failure, timeout, retry exhaustion).
- Remote onboarding/probe failures.

Representative user-facing errors:

- `unsupported format: <format>`
- `run requires at least one label`
- `invalid label <value>: ...`
- `node probe failed with HTTP <code>`
- `failed to probe remote node <node_id> at <endpoint> via <transport>`

## Environment-Driven Behavior

`run` uses environment overrides:

- `TAK_LEASE_TTL_MS` for lease TTL
- `TAK_LEASE_POLL_MS` for pending poll interval
- `TAK_SESSION_ID` optional session identifier
- `TAK_USER` optional user override

Remote inventory is loaded from XDG config when present.

## Main Files

- `src/lib.rs`: command parser, dispatcher, and command output contracts.
- `src/main.rs`: thin binary wrapper that calls `tak::run_cli`.
