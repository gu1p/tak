# takd Architecture

## Purpose

`takd` is Tak's long-lived scheduler, execution coordinator, run store, inventory owner, and remote
worker service. `takd serve` exposes the local unix-socket protocol v2 and the direct/Tor worker
surfaces from one process.

All execution is daemon-owned. takd schedules local host, local container, direct remote, and Tor
remote jobs; it also owns retries, cancellation, attempt fencing, artifacts, and persisted events.

## Submission and scheduling

The client submits a fully resolved `RunSubmission`: graph, tasks, jobs, concrete candidates,
resources, sessions, affinity, retries, queues, limiter claims, context manifest, declared outputs,
and allowlisted environment values. takd validates the projection before it commits a run.

The scheduler then:

1. finds ready jobs while preserving dependency and failure semantics;
2. applies submitter/run fairness, queue policy, limiters, and backoff;
3. checks affinity, live node capacity, transport, and placement policy;
4. reserves an attempt transactionally;
5. dispatches through local or remote attempt transport;
6. persists observations, output chunks, and terminal settlement; and
7. releases reservations and wakes newly ready work.

Balanced placement scores projected dominant CPU/memory/slot pressure and queue depth with bounded
cache/affinity locality credit. Round-robin cursors are persisted. Sequential placement starts at
the first authored candidate.

## Attempt safety

Every attempt has a generation and fencing token. Late observations from replaced attempts cannot
commit state or artifacts. Unknown/node-loss outcomes retry only when the resolved job is
idempotent; unsafe unknown outcomes fail terminally. Authored task failures use the authored retry
policy, while cancellation never fails over.

The first persisted cancellation marks unfinished work cancelling/skipped and sends cancellation to
active transports. Repeated cancellation is idempotent. Client disconnection alone has no lifecycle
effect.

## Sessions and caches

- Workspace reuse prepares an isolated snapshot for each job.
- Paths reuse restores selected paths from the daemon's private content-addressed cache and
  publishes a new generation only after success.
- SharedWorkspace reuse keeps mutable state on one hard-affinity node, enforces
  `max_parallel_tasks`, and publishes successful updates atomically.
- Container reuse fuses compatible tasks into one scheduled job/container.

Cache paths are not result artifacts. Outputs are accepted only from their declared producer and
only after path, type, digest, generation, and fencing validation.

## Protocol surfaces

The local newline-delimited JSON protocol v2 supports:

- submit a run;
- get/list runs;
- attach after an event cursor;
- cancel a run; and
- read output manifests and artifact chunks.

Remote worker protocol v2 supports signed/identified probe, dispatch, observation paging,
acknowledgement, cancellation, and output chunks over direct or Tor transport. takd owns both
inventories and never accepts a client-supplied arbitrary endpoint as final placement authority.

## Persistence and retention

The SQLite run store records immutable submissions plus mutable run/job/attempt state, events,
fences, limiter/queue state, placement cursors, cancellation, and artifact metadata. Blob content
uses a private CAS rooted beside the database.

Startup migration/recovery runs before the server accepts requests. Active attempts without valid
transport evidence are reconciled through the same idempotency and fencing rules as live node loss.

Default maintenance keeps terminal logs, outputs, and workspace/path blobs for 7 days, keeps
terminal metadata for 30 days, and limits the workspace/path blob cache to 20 GiB. Expiry is
reported explicitly by get/attach/output APIs.

## Inventory and security

The worker registry merges local capacity with direct and Tor peer snapshots, rejects incompatible
protocol versions, and treats stale/lost nodes conservatively. The Tor peer manager preserves its
last-good inventory across malformed reloads and uses warm broker sessions.

- The Tor invite/address is a secret, not just a location.
- Anyone with it can submit jobs and read outputs/logs.
- Do not paste it into shared chats, issue trackers, screenshots, or logs.
- Rotate the onion address if exposed.
- Tak remote does not provide multi-user isolation.

## Main files

- `src/daemon/run_store.rs`: durable run, attempt, scheduling, event, and artifact state.
- `src/daemon/scheduler.rs`: fair ready-job reservation loop.
- `src/daemon/attempt_coordinator.rs`: dispatch/observe/cancel settlement.
- `src/daemon/local_attempt_transport.rs`: daemon-owned local execution.
- `src/daemon/remote_attempt_transport.rs`: remote v2 attempt coordination.
- `src/daemon/worker_registry.rs`: local/direct/Tor worker snapshots and capacity.
- `src/daemon/protocol/v2_dispatch.rs`: local run protocol operations.
- `src/daemon/remote/route_worker_v2.rs`: remote worker protocol operations.
