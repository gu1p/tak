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
| `tak exec -- <program> [args...]` | "Run one tool-native command through Tak" | synthetic one-step task + execution override resolution + `run_resolved_task(...)` | Streams wrapped command stdout/stderr live and exits with the wrapped command's exit code. |
| `tak status [--node <id>...] [--watch] [--interval-ms N]` | "What is local Tak doing, and what are my configured remotes doing?" | XDG task history + optional `TAKD_SOCKET` status + `/v1/node/status` per matching remote | Local section plus remote node, container, and active-job sections; watch mode repeats snapshots. |
| `tak local status [--watch] [--interval-ms N]` | "What is this local Tak client doing?" | XDG task history + local CPU/RAM/storage + optional `TAKD_SOCKET` status | Local resource line plus container and active-job sections. |
| `tak remote add <token>` | "Add a remote execution agent" | secret invite/token decode + `/v1/node/info` probe (bounded retry for Tor onion remotes) + config write | `added remote <node_id>`. |
| `tak remote add` | "Add a remote execution agent interactively" | method picker + word/token/secret Tor invite input + probe + confirmation before config write | Interactive TUI; final success line is `added remote <node_id>`. |
| `tak remote add --words [word...]` | "Add a Tor remote execution agent by manual typing" | provided words stay non-interactive; empty `--words` opens the word-entry TUI; both use the same probe/confirmation/write path as appropriate | `added remote <node_id>`. |
| `tak remote scan` | "Scan a remote execution agent from a QR code" | camera enumeration + live preview + QR decode + existing remote-add probe/write path | Interactive TUI; final success line is `added remote <node_id>`. |
| `tak remote list` | "Which remote execution agents are configured?" | config read | One configured agent per line. |
| `tak remote status [--node <id>...] [--watch] [--interval-ms N]` | "What is each configured remote node doing right now?" | config read + `/v1/node/status` fetch per matching remote | Node, container, and active-job sections; terminal watch mode uses a Ratatui dashboard. |
| `tak remote logs --node <id> [--all|--lines N]` | "What is this remote node's daemon log?" | config read + `/v1/node/logs` fetch | Raw remote service log bytes on stdout. |
| `tak remote tasks --node <id> [--active] [--limit N]` | "Which task attempts does this remote node know about?" | config read + `/v1/tasks` fetch | `Remote Tasks` section with node, task label, task run id, attempt, and state. |
| `tak remote task logs --node <id> <task-run-id>` | "What did this task emit on that remote node?" | config read + `/v1/tasks/<id>/events` polling | Remote stdout chunks to stdout and stderr chunks to stderr. |
| `tak task list [--limit N]` | "Which task runs did this local Tak client start?" | XDG task history SQLite read | `Local Tasks` section with task label, task run id, attempts, state, placement, and remote node. |
| `tak task logs <task-run-id>` | "What did this locally initiated task emit?" | XDG task history SQLite read | Captured stdout chunks to stdout and stderr chunks to stderr. |

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
- Streams task `stdout` and `stderr` live to the local terminal for local host, local containerized, and remote containerized execution.
- Per-task status is printed after execution summary is available.

### `exec`

- Does not require a `TASKS.py` workspace.
- Builds one synthetic command task and reuses the same executor/runtime override path as `run`.
- Streams wrapped command `stdout` and `stderr` live without adding a summary line to stdout.
- Preserves the wrapped process exit code on command failure.

### `web`

- Uses embedded frontend assets and vendored graph library files.
- Binds `127.0.0.1` on a random available port.
- Prints URL and serves graph UI until interrupted.
- Auto-opens browser only in production-style runtime (`!debug_assertions`) and when `TAK_NO_BROWSER_OPEN` is not set.
- If open fails, command keeps serving and prints the manual URL.

### `status`

- Combines `tak local status` with remote node snapshots from enabled remotes.
- Missing local daemon status is reported as `daemon=unavailable` rather than failing the command.
- Non-terminal output is plain and section-oriented for scripts; terminal remote watch uses Ratatui.

### `local status`

- Reads active local task metadata from `$XDG_STATE_HOME/tak/tasks.sqlite`.
- Samples local CPU, RAM, and storage through `sysinfo`.
- Queries the optional local lease daemon through `TAKD_SOCKET` and reports unavailable daemon state as a warning field.

### `remote status`

- Queries enabled remotes from client inventory, or the selected subset passed via `--node`.
- Uses remote v1 authenticated HTTP to fetch running jobs and node resource usage.
- One-shot mode prints `Nodes`, `Containers`, and `Active Jobs` sections.
- Terminal watch mode refreshes a Ratatui dashboard at the requested interval and restores the screen on clean interrupt.

### `remote logs`, `remote tasks`, and `remote task logs`

- Require an explicit `--node` and use the configured remote inventory to resolve transport, endpoint, and bearer token.
- Direct and Tor transports share the same request path as remote status probing.
- `remote logs` reads takd service logs; task stdout/stderr stays under the task-centered commands.

### `task list` and `task logs`

- `tak run` records locally initiated task metadata and output chunks under `$XDG_STATE_HOME/tak/tasks.sqlite`.
- Run summary lines include `task_run_id=<id>` so the follow-up `tak task logs <id>` command is copyable.
- Local task logs work for local and remote placements because they replay output captured by the initiating client.

### `remote scan`

- Linux-first camera onboarding flow with a pick-camera screen, live terminal preview, and confirmation step before config writes.
- Reuses the same token decode, node probe, and inventory persistence contract as `tak remote add`.
- Non-interactive terminals are rejected with a clear error.

### `remote add`

- No-argument mode opens a terminal method picker for word entry or secret token/invite paste.
- `--words` with no following values skips the picker and opens the word-entry screen.
- The word-entry screen uses numbered cells, validates dictionary words immediately, supports undo, and checks the phrase checksum before probing.
- Interactive add confirms probed node details before writing client inventory.

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

`run` and `exec` use environment overrides:

- `TAKD_SOCKET` optional lease daemon socket path
- `TAK_LEASE_TTL_MS` for lease TTL
- `TAK_LEASE_POLL_MS` for pending poll interval
- `TAK_SESSION_ID` optional session identifier
- `TAK_USER` optional user override

Remote inventory is loaded from XDG config when present.

## Main Files

- `src/lib.rs`: command parser, dispatcher, and command output contracts.
- `src/main.rs`: thin binary wrapper that calls `tak::run_cli`.
