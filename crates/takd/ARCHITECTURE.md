# takd Architecture

## Purpose

`takd` is the machine-wide coordination daemon that arbitrates limiter leases for concurrent Tak executions.

It exposes a unix-socket NDJSON protocol and persists lease state/history in SQLite for restart recovery.

The remote execution agent mode also keeps live task state in process and persists submitted task
attempts, task events, and terminal results in `agent.sqlite` under the takd state root.

## Protocol Surface

Requests:

- `AcquireLease`
- `RenewLease`
- `ReleaseLease`
- `Status`

Responses:

- `LeaseGranted`
- `LeasePending`
- `LeaseRenewed`
- `LeaseReleased`
- `StatusSnapshot`
- `Error`

Transport:

- one JSON request per line
- one JSON response per line
- unix domain socket

## LeaseManager Semantics

- Maintains per-limiter capacities and current usage.
- Tracks active leases and pending acquisition requests.
- Acquisition is all-or-none over requested `needs`.
- Expiration reclaims usage and writes history.
- Renew updates ttl and persisted expiry.

## Persistence Model

SQLite tables:

- `active_leases`
  - restart-time snapshot of currently active leases
- `lease_history`
  - append-only event log (`acquire`, `renew`, `release`, `expire`)
- `submit_attempts`
  - remote task attempts keyed by `(task_run_id, attempt)`
- `submit_events`
  - ordered task output and lifecycle events
- `submit_results`
  - terminal task results; used with `submit_attempts` for restart abandonment and task logs

Live task listing:

- `takd serve` binds `<state_root>/agent-control.sock`
- `takd tasks` queries that socket for `/v1/node/status` and renders in-memory active jobs
- persisted unfinished submit rows are not treated as currently executing tasks after restart

Startup recovery:

1. ensure schema exists
2. load active leases
3. discard expired rows
4. rebuild in-memory usage from non-expired leases
5. mark unfinished remote task attempts abandoned before accepting new work

## Capacity and Queue Behavior

- Capacity key: `(name, scope, scope_key)`.
- Requests exceeding available capacity are returned as pending.
- Pending queue position is surfaced to clients.
- Releasing/expiring leases frees capacity for subsequent requests.

## Failure Classes

- malformed/invalid request payload
- acquire request with missing/invalid needs
- renew/release for unknown lease id
- socket bind/connect/read/write errors
- sqlite schema/open/transaction failures

## Operational Entry Points

- `run_server(socket_path, manager)`
- `run_daemon(socket_path)` default daemon bootstrap
- `default_socket_path()` and `default_state_db_path()`
- `takd tasks`: list live remote task attempts from the running local takd process.
- `takd task logs <task-run-id> [--follow]`: print persisted stdout/stderr chunks for a task run.

## Main Files

- `src/lib.rs`: protocol definitions, request dispatch, lease manager, sqlite integration.
- `src/main.rs`: executable entrypoint for daemon process.
