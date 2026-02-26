# tak CLI Architecture

## Purpose

`tak` is the user-facing interface over three subsystems:

- `tak-loader` for workspace/task graph discovery
- `tak-exec` for task execution
- `takd` for machine-wide lease status and daemon lifecycle

The CLI is query + action oriented: each command answers one operational question and prints a stable text response contract.

## Runtime Shape

- `src/main.rs`: process entrypoint, delegates to library runtime.
- `src/lib.rs`: clap parsing, command dispatch, output formatting, and daemon status RPC.

High-level flow:

1. Parse command with clap.
2. For workspace commands, load `WorkspaceSpec` from current working directory.
3. Dispatch to loader/executor/daemon APIs.
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
| `tak status` | "What is daemon lease state right now?" | `query_daemon_status(...)` NDJSON RPC | `active_leases`, `pending_requests`, then `usage ...` lines. |
| `tak daemon start` | "Start a local coordination daemon" | `takd::run_daemon(...)` | No structured success payload; process becomes daemon server loop. |
| `tak daemon status` | "Is daemon healthy and what is queue pressure?" | same RPC as `status` | `active_leases` + `pending_requests`. |

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

### `status` and `daemon status`

- Both depend on daemon socket connectivity.
- Output values come from daemon `StatusSnapshot`:
  - active lease count
  - pending request count
  - optional limiter usage rows (only `status` prints full usage rows)

### `daemon start`

- Binds unix socket and runs daemon loop indefinitely.
- Intended to run in its own terminal/session.

## Error and Exit Semantics

All commands fail fast on errors and return non-zero exit status.

Common failure classes:

- Workspace load/parse errors (`tak.toml`, `TASKS.py`, label resolution).
- Invalid CLI input (unsupported graph format, missing run labels, bad label syntax).
- Execution failures (`run` task failure, timeout, retry exhaustion).
- Daemon RPC failures (socket not reachable, protocol error, daemon-side error response).

Representative user-facing errors:

- `unsupported format: <format>`
- `run requires at least one label`
- `invalid label <value>: ...`
- `failed to connect to daemon at <socket>`
- `daemon error: <message>`

## Environment-Driven Behavior

`run` and daemon-status paths use environment overrides:

- `TAKD_SOCKET` for daemon socket path
- `TAK_LEASE_TTL_MS` for lease TTL
- `TAK_LEASE_POLL_MS` for pending poll interval
- `TAK_SESSION_ID` optional session identifier
- `TAK_USER` optional user override

If unset, the CLI uses built-in defaults and daemon default socket resolution.

## Main Files

- `src/lib.rs`: command parser, dispatcher, command output contracts, daemon status RPC.
- `src/main.rs`: thin binary wrapper that calls `tak::run_cli`.
