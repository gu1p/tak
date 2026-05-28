# PLAN_FINAL.md: Tor-First Live `takd` Peer Network

## Summary

`takd serve` becomes the long-lived connectivity layer for Tor remote execution while still serving as the remote agent entrypoint.

There is one canonical service command:

- `takd serve`

That command encompasses both roles:

- Local daemon/broker role: Unix socket, peer manager, warm Tor sessions, placement, submit, streaming, cancel, result, and output transport.
- Remote agent role: accepts and executes work from other `takd` instances.

`tak` must continue to run without `takd` for local execution and other workflows that do not require daemon-owned Tor remote execution. Tor remote execution is daemon-owned in this plan. If Tor remote execution is requested and local `takd serve` is unavailable, `tak run` fails clearly instead of opening its own Tor connection.

Direct daemon-owned remote execution is future work. Direct remotes remain outside the v1 peer-manager scope and keep the existing direct-status behavior.

## Current Problems

Remote Tor execution currently has too much client-owned lifecycle:

- `tak` and `tak-exec` read remote inventory for execution decisions.
- `tak` and `tak-exec` select and probe remote nodes during each run.
- `tak` and `tak-exec` can open Tor connections during a run.
- The local daemon can broker Tor traffic, but the broker is request-scoped and client-directed.
- There is no daemon-owned peer state, heartbeat loop, or Tor placement API.

This makes Tor remote execution slower and less reliable than necessary. Connectivity that should be warm and supervised by a long-lived local service is rediscovered by each run.

## Target Architecture

Target flow:

```text
tak run ...
  -> local takd serve over Unix socket
      -> local execution when placement is local
      -> warm Tor peer session when placement is remote Tor
          -> remote takd serve agent
```

`takd serve` owns:

- local Unix-socket daemon APIs
- local resource coordination
- remote inventory loading for Tor peers
- Tor peer session lifecycle
- heartbeat and reconnect backoff
- peer health and live load state
- Tor remote placement decisions
- Tor remote submit, cancel, event, result, and output transport
- authentication to Tor peers
- direct operation as a remote execution agent

`tak` owns:

- CLI parsing and user interaction
- loading the workspace and task graph
- local execution when no daemon-owned remote execution is needed
- sending Tor remote execution requests to local `takd serve`
- rendering streamed events, output, and results returned by local `takd serve`

`tak` must not send arbitrary Tor endpoint details to local `takd` as proxy instructions. It sends placement requirements, task metadata, and workspace staging inputs. Local `takd serve` resolves eligible Tor peers from its inventory view.

## Command Surface

Canonical commands:

```text
takd serve
takd peers
tak status
tak remote status
```

Rules:

- `takd serve` is the only service command.
- Do not keep `takd daemon serve` as a compatibility alias.
- `takd peers` reports local daemon peer state.
- `tak status` and `tak remote status` use daemon peer snapshots for Tor remotes when local `takd serve` is reachable.
- Direct remotes keep the existing direct-status path. Mixed direct+Tor inventories combine direct probe results with daemon Tor peer snapshots.
- If local `takd serve` is not reachable, `tak status` may still report local status and may show Tor peer state as unavailable.
- `tak run --local ...` and local-only task execution continue to work without `takd`.

Example `takd peers` output:

```text
NODE         TRANSPORT  STATE        LAST_HEARTBEAT  JOBS  QUEUE
builder-a    tor        connected    2s ago          1     0
builder-b    tor        degraded     34s ago         ?     ?
builder-c    tor        auth_failed  never           ?     ?
```

## Inventory Ownership

Keep the existing inventory file:

```text
~/.config/tak/remotes.toml
```

Rules:

- `tak remote add` continues writing this file.
- Move inventory parsing and path resolution into shared code used by `tak`, `tak-exec`, and `takd`.
- `takd serve` reads this file on startup.
- `takd serve` reloads this file while running.
- For v1, use periodic hash or mtime polling with debounce-like behavior.
- Reload is last-good: a malformed inventory update must not drop existing peers.
- `takd serve` never invents remote nodes.
- Live peer state is in memory for v1 and is never written back to `remotes.toml`.

Reload behavior:

- new enabled Tor remote -> spawn peer task
- disabled or removed Tor remote -> close session, remove it from peer snapshots, and exclude it from placement
- changed bearer token -> clear sticky `auth_failed` and reconnect
- changed endpoint or transport -> clear sticky `auth_failed`, tear down, and rebuild
- direct remote -> leave out of v1 Tor peer manager

## Peer Manager

`takd serve` starts a `PeerManager` for the lifetime of the process.

The peer manager:

- loads enabled Tor remotes from inventory
- starts one supervised task per enabled Tor peer
- establishes warm HTTP/2 sessions over Arti streams
- sends periodic app-level heartbeat requests
- tracks auth failures separately from transport failures
- reconnects dropped peers with exponential backoff and jitter
- exposes peer snapshots to local daemon APIs
- excludes unhealthy and auth-failed peers from Tor placement
- closes sessions and removes snapshots when inventory disables or removes a peer

Reuse the existing `TorBroker` HTTP/2 session pool internally. Do not rebuild transport from scratch.

Peer states:

- `disconnected`: enabled peer has no active session and is waiting for supervision to reconnect
- `connecting`: attempting initial connection or reconnect
- `connected`: heartbeat and protocol checks are healthy
- `degraded`: session was recently healthy, but one heartbeat failed or timed out
- `auth_failed`: peer rejected credentials; do not retry until token, endpoint, or transport changes
- `unreachable`: repeated transport or heartbeat failures after retry/backoff
- `protocol_mismatch`: peer does not support the required v1 ping/protocol contract

State includes:

- node id
- display name
- transport
- endpoint
- state
- last heartbeat time
- last successful connection time
- last error summary
- active job count
- queue depth
- compact resource summary
- protocol version
- heartbeat RTT
- reconnect attempt count

## Heartbeats

The remote agent is a stateless HTTP/2 REST server, so heartbeat is client-initiated by local `takd serve`.

Add:

```text
GET /v1/node/ping
```

Add a `NodePingResponse` protobuf message with:

- node id
- protocol version
- health
- active job count
- queue depth
- compact resource summary

Defaults:

- heartbeat interval: 15 seconds
- `connected -> degraded`: 1 failed or timed-out ping
- `degraded -> unreachable`: 2 consecutive failed or timed-out pings
- reconnect backoff start: 1 second
- reconnect backoff max: 60 seconds
- jitter: +/- 20%
- reset backoff after successful ping
- `auth_failed` on HTTP 401/403 or equivalent protocol auth rejection
- `protocol_mismatch` on HTTP 404/501 for `/v1/node/ping` or invalid ping protobuf

Heartbeat, submit, cancel, result, and output requests use the bearer token from inventory. Tokens must never appear in peer snapshots, command output, or logs.

`auth_failed` is sticky. It leaves that state when the inventory bearer token, endpoint, or transport for that node changes.

`protocol_mismatch` peers are excluded from placement and retried with backoff so upgraded agents can recover.

HTTP/2 PING keepalive can be added later. The v1 contract is app-level `/v1/node/ping`.

## Local Daemon Protocol

Extend the local Unix-socket protocol beyond lease/status messages.

Add daemon-owned JSON-RPC-style requests:

- `PeersList`: return peer snapshots
- `PeersEligible`: filter connected Tor peers by pool, tag, capability, transport, and requested resource shape
- `PlaceRemote`: select a Tor peer, stage/upload the workspace as needed, submit work over the warm session, and return a daemon task handle
- `StreamTaskEvents`: stream lifecycle and output events for a daemon task handle
- `CancelTask`: cancel local or remote task
- `GetTaskResult`: return terminal result
- `GetOutputRange`: fetch output artifact byte ranges

`tak` sends placement requirements, task metadata, and workspace staging inputs. The daemon resolves node id, endpoint, transport, auth, upload, submit, events, cancel, result, and output ranges from inventory and daemon state.

The final Tor execution path must not depend on client-supplied endpoint forwarding headers such as `X-Tak-Remote-Endpoint`.

V1 daemon task handles are local-daemon in-memory handles. If local `takd serve` restarts during an active Tor run, `tak` receives a clear daemon-lost failure. Resumable daemon task handles are future work.

Large binary transfers must use a framed or HTTP-compatible range path instead of loading unbounded output into memory.

## Remote Peer Protocol

Remote `takd serve` must support:

- `GET /v1/node/ping`
- `GET /v1/node/status`
- `POST /v1/tasks/submit`
- workspace upload endpoints
- event polling/streaming endpoints
- result fetch
- cancel
- output artifact range fetch

Only `/v1/node/ping` is new for v1. Existing remote task, workspace, event, result, cancel, and output endpoints should be reused where practical.

## Execution Flow

Tor remote execution:

```text
tak run ...
  -> connect to local takd serve socket
  -> send task request and placement requirements
  -> local takd serve selects eligible Tor peer
  -> local takd serve submits over existing warm HTTP/2 session
  -> local takd serve streams events/output/result back to tak
```

If local `takd serve` is unavailable and Tor remote execution is required:

- fail clearly
- explain that Tor remote execution requires local `takd serve`
- do not bootstrap client-side Tor
- do not fall back to legacy client-owned Tor execution

If local execution is requested or selected:

- `tak` continues to run without requiring `takd`
- `tak run --local ...` remains valid without a daemon

## Status And Diagnostics

`takd peers` shows daemon peer state.

`tak status` and `tak remote status` use daemon peer snapshots for Tor remotes when local `takd serve` is reachable. Direct remotes continue to use the existing direct-status path and are shown alongside Tor peer snapshots in mixed inventories.

Diagnostics should distinguish:

- no local daemon for Tor remote execution
- no configured Tor peers
- all peers unreachable
- all peers auth failed
- peer protocol mismatch
- no pool/tag/capability match
- resource/load mismatch

Example:

```text
local takd reachable: /run/user/1000/tak/takd.sock
peer builder-a: connected via tor, last heartbeat 2s ago
peer builder-c: auth_failed, excluded from placement
```

## Non-Goals

This plan does not include:

- direct daemon-owned remote execution
- peer-to-peer distribution between remote agents
- gossip discovery
- NAT traversal beyond existing Tor/direct transports
- remote nodes discovering each other
- replacing `tak remote add`
- preserving `takd daemon serve`
- preserving legacy client-side Tor fallback for Tor remote execution
- resumable daemon task handles across local `takd serve` restart

## Phased Delivery

Each phase must follow strict TDD and be independently reviewable.

1. Move inventory parsing/path resolution into shared code and add last-good reload.
2. Add `/v1/node/ping`, `NodePingResponse`, binary/proto contract coverage, and protocol-mismatch handling.
3. Refactor `takd serve` into the single service entrypoint that owns local daemon APIs and remote agent behavior.
4. Add `PeerManager` skeleton with supervised Tor peer tasks, state model, heartbeat loop, backoff, inventory reconciliation, and `takd peers`.
5. Add the complete daemon task lifecycle APIs: `PeersList`, `PeersEligible`, `PlaceRemote`, `StreamTaskEvents`, `CancelTask`, `GetTaskResult`, and `GetOutputRange`.
6. Route Tor `tak run` through local daemon-owned placement and lifecycle APIs; remove client-side Tor bootstrap and endpoint-forwarding dependency from this path.
7. Update `tak status` and `tak remote status` to combine daemon Tor peer state with existing direct-remote status behavior.
8. Remove `takd daemon serve` compatibility, audit docs/scripts/installers/examples, and document the exact user-facing rejection/help behavior.
9. Update architecture docs.

## Required Tests

Follow the repo workflow:

1. BDD/behavioral and UI-contract tests first.
2. Unit tests second.
3. Integration tests third.
4. Implementation last.
5. Refactor only after tests are green.
6. Validate each phase/change with `tak run --local //:check`.

Required coverage:

- `takd serve` starts local daemon APIs and remote agent behavior.
- `takd daemon serve` is not part of the final CLI contract.
- `takd peers` renders empty, connecting, connected, degraded, unreachable, auth_failed, and protocol_mismatch states.
- `takd serve` loads enabled Tor remotes from `remotes.toml`.
- disabled and removed Tor remotes are removed from peer snapshots and excluded from placement.
- direct remotes are not v1 peer-manager targets.
- direct-only and mixed direct+Tor status keep direct probes while Tor peers use daemon snapshots.
- malformed inventory reload preserves last-good peer state.
- token, endpoint, or transport change clears sticky `auth_failed` and reconnects/rebuilds.
- added and removed Tor remotes are picked up without restart.
- heartbeat failure marks `connected -> degraded -> unreachable`.
- `/v1/node/ping` uses inventory auth, marks `auth_failed` on 401/403, and redacts tokens from snapshots/logs/output.
- old or incompatible remotes that return 404/501 or invalid ping protobuf become `protocol_mismatch` and are excluded from placement.
- remote restart reconnects without any `tak` process running.
- `tak run` uses daemon-owned Tor execution.
- `tak run` fails clearly for Tor remote execution when local `takd serve` is unavailable.
- `tak run --local` works without `takd`.
- `tak run` does not bootstrap client-side Tor.
- submit, events, cancel, result, and output range fetches flow through local daemon for Tor.
- Tor `tak run` does not depend on client-supplied endpoint-forwarding headers.
- active Tor run gets a clear daemon-lost failure if local `takd serve` restarts.
- output range fetch is bounded and does not load unbounded artifacts into memory.
- `tak status` and `tak remote status` combine daemon Tor peer state with existing direct-remote status when reachable.
- diagnostics distinguish no configured Tor peers, all unreachable, all auth failed, protocol mismatch, no pool/tag/capability match, and resource/load mismatch.
- `NodePingResponse` has a binary/proto contract test.

Final validation command:

```text
tak run --local //:check
```

## Acceptance Criteria

The feature is complete when:

- Starting `takd serve` starts the local daemon socket and Tor peer manager.
- Starting `takd serve` on a configured node also serves remote-agent work.
- Adding Tor remotes through `tak remote add` causes local `takd serve` to connect without any task run.
- `takd peers` shows Tor peers moving from `connecting` to `connected`.
- Logs show daemon-owned `/v1/node/ping` traffic at the configured cadence.
- Killing a remote agent changes peer state without invoking `tak`.
- Restarting the remote returns it to `connected` automatically.
- Tor `tak run` reuses a warm daemon-owned peer session.
- Tor `tak run` opens no client-side Tor connection.
- Tor `tak run` does not send or require client-supplied endpoint-forwarding headers.
- Tor `tak run` fails clearly when local `takd serve` is unavailable.
- Tor run submit, events, cancel, result, and output range fetches all flow through local `takd serve`.
- Incompatible Tor peers are visible as protocol mismatches and excluded from placement.
- Direct remotes continue to appear through the existing direct-status path.
- `tak run --local ...` works without local `takd serve`.
- `tak run --local //:check` passes in the final branch state.
