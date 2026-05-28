# takd Architecture

## Purpose

`takd` is the long-lived connectivity and coordination daemon for Tak.

`takd serve` is the single service entrypoint. It starts the local unix-socket daemon APIs, the
PeerManager for Tor remotes, the local Tor broker/session pool, and the remote execution agent.
It persists lease state/history in SQLite for restart recovery.

The remote execution agent mode also keeps live task state in process and persists submitted task
attempts, task events, and terminal results in `agent.sqlite` under the takd state root.

## Tor Remote Security Model

The Tor invite/address is a secret, not just a location.
Anyone with it can submit jobs and read outputs/logs.
Do not paste it into shared chats, issue trackers, screenshots, or logs.
Rotate the onion address if exposed.
Tak remote does not provide multi-user isolation.

## Protocol Surface

Requests:

- `AcquireLease`
- `RenewLease`
- `ReleaseLease`
- `Status`
- `PeersList`
- `PeersEligible`
- `PlaceRemote`
- `StreamTaskEvents`
- `CancelTask`
- `GetTaskResult`
- `GetOutputRange`

Responses:

- `LeaseGranted`
- `LeasePending`
- `LeaseRenewed`
- `LeaseReleased`
- `StatusSnapshot`
- `PeersSnapshot`
- `RemotePlaced`
- `Error`

Transport:

- one JSON request per line
- one JSON response per line
- unix domain socket

Remote agent HTTP endpoints:

- `/v1/node/ping`, returning `NodePingResponse` for PeerManager heartbeats
- `/v1/node/status`
- `/v1/tasks/submit`
- workspace upload, task event, cancel, result, and output range endpoints

`takd daemon serve` is not part of the CLI contract; `takd serve` owns both local daemon APIs and
remote-agent behavior.

## PeerManager Semantics

- Loads enabled Tor remotes from `remotes.toml`.
- Excludes disabled records and direct remotes from the v1 Tor peer manager.
- Maintains in-memory states: `connecting`, `connected`, `degraded`, `unreachable`,
  `auth_failed`, and `disconnected`.
- Uses the TorBroker HTTP/2 session pool for warm peer traffic instead of rebuilding transport.
- Sends app-level `/v1/node/ping` heartbeats and records load, protocol version, resource summary,
  and heartbeat RTT.
- Preserves the last-good peer set when an inventory reload is malformed.
- Clears sticky `auth_failed` only when the inventory bearer token changes.
- Serves `takd peers` from the local daemon `PeersList` response.

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
- in-memory Tor peer snapshots
  - loaded from `remotes.toml`, never persisted back to inventory

Live task listing:

- `takd serve` binds `<state_root>/agent-control.sock`
- `takd tasks` queries that socket for `/v1/node/status` and renders in-memory active jobs
- persisted unfinished submit rows are not treated as currently executing tasks after restart
- remote clients may query `/v1/node/logs` for the daemon `service.log`
- remote clients may query `/v1/tasks` for persisted submit attempt summaries

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
- malformed inventory reloads, which preserve last-good peer state
- no configured Tor peers, no eligible peer, unreachable peer, or sticky `auth_failed` peer state

## Operational Entry Points

- `run_server(socket_path, manager)`
- `run_daemon(socket_path)` default daemon bootstrap
- `default_socket_path()` and `default_state_db_path()`
- `takd serve`: unified local daemon, PeerManager, Tor broker, and remote agent service.
- `takd peers`: render daemon-owned Tor peer state.
- `takd tasks`: list live remote task attempts from the running local takd process.
- `takd task logs <task-run-id> [--follow]`: print persisted stdout/stderr chunks for a task run.
- remote v1 `/v1/node/logs`: read the node service log with `all=true` or `lines=N`.
- remote v1 `/v1/tasks`: list active or persisted task attempt summaries.

## Main Files

- `src/lib.rs`: protocol definitions, request dispatch, lease manager, sqlite integration.
- `src/main.rs`: executable entrypoint for daemon process.
