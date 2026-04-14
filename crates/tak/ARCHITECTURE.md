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
| `tak list` | "What tasks exist in this workspace?" | `load_workspace_from_cwd()` | ANSI-decorated list output with fully-qualified labels from the current directory workspace. |
| `tak explain <label>` | "What is this task composed of?" | workspace load + guided label parse + task lookup | Structured text fields: `label`, `deps`, `steps`, `needs`, `timeout_s`, `retry_attempts`. |
| `tak graph [label] --format dot` | "What dependency graph should I visualize?" | workspace load + optional guided label parse | DOT graph text (`digraph tak { ... }`). |
| `tak web [label]` | "Show this graph interactively in browser" | workspace load + optional guided label parse + embedded local server | Prints local URL, serves embedded HTML/CSS/JS UI, runs until `Ctrl+C`. |
| `tak run <label...> [-j N] [--keep-going]` | "Execute these targets with dependencies" | workspace load + guided label parsing + `run_tasks(...)` | One result line per executed label with attempts, exit, placement, remote, transport, reason, context hash, and runtime fields. |
| `tak status` | "Is live coordination status available here?" | none in the current client-only build | Returns an unsupported error. |
| `tak remote add <token>` | "Add a remote execution agent" | token decode + `/v1/node/info` probe (bounded retry for Tor onion remotes) + config write | `added remote <node_id>`. |
| `tak remote scan` | "Scan a remote execution agent from a QR code" | camera enumeration + live preview + QR decode + existing remote-add probe/write path | Interactive TUI; final success line is `added remote <node_id>`. |
| `tak remote list` | "Which remote execution agents are configured?" | config read | One configured agent per line. |
| `tak remote status [--node <id>...] [--watch] [--interval-ms N]` | "What is each configured remote node doing right now?" | config read + `/v1/node/status` fetch per matching remote | Node summary section plus active-job section; watch mode refreshes the snapshot in place. |

## Output Details Per Command

### `list`

- Success output: one human-readable line per task with canonical labels and dependency hints.
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
- Rejects path-like inputs such as `.`, `./foo`, or `/tmp/task` and points users to `tak list`.
- Delegates retry, timeout, and lease behavior to `tak-exec`.
- Streams task `stdout` and `stderr` live to the local terminal for local, remote, and containerized execution.
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

### `remote status`

- Queries enabled remotes from client inventory, or the selected subset passed via `--node`.
- Uses remote v1 authenticated HTTP to fetch running jobs and node resource usage.
- One-shot mode prints a `Nodes` section and an `Active Jobs` section.
- Watch mode refreshes stdout in place at the requested interval.

### `remote scan`

- Linux-first camera onboarding flow with a pick-camera screen, live terminal preview, and confirmation step before config writes.
- Reuses the same token decode, node probe, and inventory persistence contract as `tak remote add`.
- Non-interactive terminals are rejected with a clear error.

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
- `<value>` is not a valid task label
- `node probe failed with HTTP <code>`
- `failed to probe remote node <node_id> at <endpoint> via <transport>`

## Environment-Driven Behavior

`run` uses environment overrides:

- `TAKD_SOCKET` optional lease daemon socket path
- `TAK_LEASE_TTL_MS` for lease TTL
- `TAK_LEASE_POLL_MS` for pending poll interval
- `TAK_SESSION_ID` optional session identifier
- `TAK_USER` optional user override

Remote inventory is loaded from XDG config when present.

## Main Files

- `src/lib.rs`: command parser, dispatcher, and command output contracts.
- `src/main.rs`: thin binary wrapper that calls `tak::run_cli`.
