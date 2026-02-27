# Tak Remote V1 Refactor (Constructor Model)

This file is the V1 authority. If it conflicts with `PLAN_REMOTE_NEW.md`, this file wins for V1.

V1 is centered on three task execution constructors:

1. `LocalOnly(Local)`
2. `RemoteOnly(Remote | list[Remote])`
3. `ByCustomPolicy(policy_fn)`

`ByCustomPolicy` receives only a function. It does not receive `RemoteOnly` or `LocalOnly`.
The function gets context that includes local and remote objects.

## 1. Canonical `TASKS.py` Shape (Normative)

```python
from tak import cmd, module_spec, task, path, gitignore
from tak.remote import (
    Local, Remote,
    LocalOnly, RemoteOnly, ByCustomPolicy,
    PolicyContext, Decision, Reason,
    CurrentState,
    WorkspaceTransferMode, ResultSyncMode, results,
    RemoteTransportMode, ServiceAuth,
)

LOCAL = Local(
    id="dev-macbook",
    max_parallel_tasks=4,
)

REMOTE_ARM = Remote(
    id="build-arm64-us",
    transport=RemoteTransportMode.TorOnionService(
        endpoint="http://abc123def456.onion",
        auth=ServiceAuth.from_env("TAK_NODE_BUILD_ARM64_US_TOKEN"),
    ),
    workspace=WorkspaceTransferMode.REPO_ZIP_SNAPSHOT,
    result=results(sync=ResultSyncMode.OUTPUTS_AND_LOGS),
)

REMOTE_X86 = Remote(
    id="build-x86-us",
    transport=RemoteTransportMode.DirectHttps(
        endpoint="https://build-x86-us.internal",
        auth=ServiceAuth.from_env("TAK_NODE_BUILD_X86_US_TOKEN"),
    ),
    workspace=WorkspaceTransferMode.REPO_ZIP_SNAPSHOT,
    result=results(sync=ResultSyncMode.OUTPUTS_AND_LOGS),
)

def choose_runtime(ctx: PolicyContext) -> Decision:
    if ctx.task.side_effecting:
        return Decision.local(reason=Reason.SIDE_EFFECTING_TASK)

    if not ctx.remote_any_reachable:
        return Decision.local(reason=Reason.NO_REMOTE_REACHABLE)

    arm = ctx.remote("build-arm64-us")
    if (
        ctx.local.cpu_percent >= 85
        and arm
        and arm.healthy
        and arm.queue_eta_s < 20
    ):
        return Decision.remote(
            "build-arm64-us",
            reason=Reason.LOCAL_CPU_HIGH_ARM_IDLE,
        )

    if ctx.local.cpu_percent >= 85:
        return Decision.remote_any(
            ["build-arm64-us", "build-x86-us"],
            reason=Reason.LOCAL_CPU_HIGH,
        )

    return Decision.local(reason=Reason.DEFAULT_LOCAL_POLICY)

SPEC = module_spec(tasks=[
    task(
        "bootstrap",
        steps=[cmd("sh", "-c", "echo bootstrap")],
        execution=LocalOnly(LOCAL),
    ),
    task(
        "unit_arm",
        deps=[":bootstrap"],
        steps=[cmd("cargo", "test", "-q", cwd="services/rust_app")],
        context=CurrentState(
            roots=[path("//services/rust_app")],
            ignored=[path("**/target/**"), gitignore()],
            include=[path("//services/rust_app/Cargo.lock")],
        ),
        execution=RemoteOnly(REMOTE_ARM),
    ),
    task(
        "unit_remote_pool",
        deps=[":bootstrap"],
        steps=[cmd("uv", "run", "pytest", "-q", cwd="services/py_app")],
        context=CurrentState(
            roots=[path("//services/py_app")],
            ignored=[path("**/.venv/**"), path("**/__pycache__/**"), gitignore()],
            include=[],
        ),
        execution=RemoteOnly([REMOTE_ARM, REMOTE_X86]),
    ),
    task(
        "build",
        deps=[":unit_arm", ":unit_remote_pool"],
        steps=[cmd("sh", "-c", "echo build")],
        execution=ByCustomPolicy(choose_runtime),
    ),
])
SPEC
```

## 2. Locked V1 Scope

1. Modes in scope:
   - local execution
   - hard remote execution
   - custom policy execution (`ByCustomPolicy`)
2. Transfer mode in scope:
   - `WorkspaceTransferMode.REPO_ZIP_SNAPSHOT` only
3. Result sync mode in scope:
   - `ResultSyncMode.OUTPUTS_AND_LOGS` only
4. Task context in scope:
   - `CurrentState(roots, ignored, include)` on tasks
   - deterministic transfer boundary: `roots -> ignored -> include`
   - persisted `ContextManifest` hash per task attempt
5. No scoring in V1:
   - no numeric weights
   - no node ranking formulas
   - no telemetry staleness scoring rules
6. Protocol in scope:
   - canonical endpoint contract: submit, events, cancel, result, node status, node capabilities
   - NDJSON event stream with monotonic sequence and resume via `after_seq`
   - idempotent submit keyed by `(task_run_id, attempt)`
7. Tor transport in scope:
   - local `takd` uses Arti to reach onion service endpoints
   - Restricted Discovery + service token auth are both supported
   - same remote protocol contract as direct HTTPS

## 3. Type Contracts (Python and IR)

### 3.1 Constructors

1. `LocalOnly(local: Local)`
2. `RemoteOnly(remote: Remote | list[Remote])`
3. `ByCustomPolicy(policy: Callable[[PolicyContext], Decision])`

### 3.2 Local

- `id: str`
- `max_parallel_tasks: int`

### 3.3 Remote

- `id: str`
- `transport: RemoteTransportMode` (`DirectHttps` or `TorOnionService`)
- `workspace: WorkspaceTransferMode` (V1 must be `REPO_ZIP_SNAPSHOT`)
- `result: ResultSpec` (V1 must resolve to `OUTPUTS_AND_LOGS`)

### 3.4 PolicyContext (minimum V1 fields)

- `task.side_effecting: bool`
- `local.cpu_percent: float`
- `remote_any_reachable: bool`
- `remote(node_id: str) -> RemoteRuntimeView | None`
- `RemoteRuntimeView.healthy: bool`
- `RemoteRuntimeView.queue_eta_s: float`

### 3.5 Decision API (no weights in V1)

Allowed:

- `Decision.local(reason=...)`
- `Decision.remote(node_id, reason=...)`
- `Decision.remote_any(node_ids, reason=...)`

Not allowed in V1:

- builder-style decision API (`Decision.start`, `prefer_*`, `require_*`, `resolve`)
- numeric scoring (`weight`, `score`, coefficients)

### 3.6 Task Context (minimum V1 fields)

- `task.context: CurrentState | None`
  - if omitted, default is `CurrentState(roots=[path("//")], ignored=[], include=[])`
- `CurrentState.roots: list[path]`
- `CurrentState.ignored: list[IgnoreSource]`
- `CurrentState.include: list[path]`

`IgnoreSource` variants:

- `path("glob-or-path")`
- `gitignore()` (optional source; if no gitignore exists, it contributes nothing)

Path forms and anchors (typed IR, not raw strings):

- `path("services/api/**")` -> package anchor
- `path("//services/api/**")` -> workspace anchor
- `path("@infra//docker/base.Dockerfile")` -> named repo anchor (`infra`)
- normalized IR shape: `PathRef { anchor, repo_id?, rel }`

Context normalization and transfer rules:

- normalize separators and collapse `.` segments
- reject escapes outside the selected anchor roots
- flatten + dedupe ignore sources before evaluation
- compute transfer set in strict order: `roots -> ignored -> include`
- `include` may re-include ignored entries but must remain inside selected roots
- resulting transfer set is persisted as `ContextManifest` and hashed

### 3.7 Remote Protocol Contract (minimum V1)

Canonical HTTP-shaped endpoints (transport-neutral semantics):

1. `POST /v1/tasks/submit` (idempotent by `(task_run_id, attempt)`)
2. `GET /v1/tasks/{task_run_id}/events?after_seq=<n>`
3. `POST /v1/tasks/{task_run_id}/cancel`
4. `GET /v1/tasks/{task_run_id}/result`
5. `GET /v1/node/status`
6. `GET /v1/node/capabilities`

Request metadata (minimum V1):

- protocol version marker (header)
- node-scoped service auth token (header)
- optional run attribution token (header)
- deterministic identity: `task_run_id`, `attempt`

Event stream contract:

- NDJSON framing
- each event carries `seq`, `task_run_id`, `type`, `timestamp_ms`, `payload`
- `seq` is monotonic per `task_run_id`
- reconnect with `after_seq=<last_seen_seq>` resumes without duplicate delivery

Result envelope contract (minimum V1):

- status/exit (`status`, `exit_code`)
- timing (`started_at`, `finished_at`, `duration_ms`)
- placement (`node_id`, `transport_kind`)
- logs/artifacts metadata (`log_artifact_uri`, output list with digest/size/path)
- bounded stdout/stderr tails and optional structured failure reason

## 4. Decision Semantics (Normative)

1. Policy runs once per task attempt.
2. Policy output is immutable for that attempt.
3. Pre-submit remote health check runs after decision, before submit.

Strict pin behavior:

- `Decision.remote("node-a")` is strict.
- If `node-a` is unavailable at submit time, result is infra error.
- No implicit fallback to another node.

Ordered fallback behavior:

- `Decision.remote_any(["node-a", "node-b"])` tries in listed order.
- First submit-capable node wins.
- If all fail, infra error.

## 5. Runtime Integration (Crate Responsibilities)

### 5.1 `tak` CLI

- unchanged user entrypoint (`tak run ...`)
- prints local and remote progress/logs/results

### 5.2 `tak-loader`

- extend Python prelude + stubs to parse:
  - `Local`, `Remote`
  - `LocalOnly`, `RemoteOnly`, `ByCustomPolicy`
  - `CurrentState`, `path(...)`, `gitignore()`
  - `Decision` helper calls
- compile custom policy function into restricted policy IR
- compile `CurrentState` into a normalized `ContextManifest`
- reject unsupported V1 shapes at load time

### 5.3 `tak-core`

- add model types:
  - `LocalSpec`
  - `RemoteSpec`
  - `TaskExecutionSpec::{LocalOnly, RemoteOnly, ByCustomPolicy}`
  - `CurrentStateSpec`
  - `PathRef`
  - `ContextManifest`
  - `PolicyContextSnapshot`
  - `PolicyDecision`
  - `ReasonCode`

### 5.4 `tak-exec`

- before each task run, evaluate `TaskExecutionSpec`
- `LocalOnly` -> existing local execution path
- `RemoteOnly` -> evaluate context + delegate remote dispatch to local `takd`
- `ByCustomPolicy` -> build context, evaluate policy IR, dispatch decision
- build remote workspace payload from `ContextManifest` only
- consume remote events and forward/persist logs through local run summary contract
- persist chosen mode/node/reason in run summary

### 5.5 `takd`

- network send/receive lives here
- local `tak` talks to local `takd`
- local `takd` talks over transport (direct or Tor) to remote `takd`
- remote nodes run `takd` only (no `tak` required)
- local `takd` owns remote handshake, stream resume, and submit idempotency handling
- remote `takd` exposes canonical V1 endpoints and event sequencing semantics
- existing lease behavior remains unchanged and backward-compatible

## 6. Transport and Node Rules (V1)

1. Supported transports:
   - direct HTTPS
   - Tor onion via Arti
2. `RemoteOnly(remote)` means one allowed node (hard remote).
3. `RemoteOnly([r1, r2, ...])` means allowed remote set with ordered fallback.
4. `ByCustomPolicy` may return strict node or ordered set.
5. For remote execution, payload content is constrained to `ContextManifest`.

### 6.1 Local `takd` -> Remote `takd` Handshake (Normative V1)

Per task attempt, local `takd` performs:

1. Receive remote dispatch request from executor with:
   - `task_run_id`, `attempt`
   - selected node (strict or fallback candidate)
   - normalized `ContextManifest` + hash
2. Build transport via `TransportFactory(RemoteTransportMode)`.
3. Preflight remote node:
   - `GET /v1/node/capabilities`
   - `GET /v1/node/status`
   - validate protocol compatibility, node identity, and basic health.
4. Submit:
   - `POST /v1/tasks/submit` with idempotency tuple `(task_run_id, attempt)`
   - include placement metadata + context manifest reference.
5. Stream:
   - open `GET /v1/tasks/{task_run_id}/events?after_seq=<last_seq>`
   - persist each event with checkpointed `last_seq`
   - forward `TASK_LOG_CHUNK` events to local logs immediately.
6. Complete:
   - `GET /v1/tasks/{task_run_id}/result`
   - validate envelope and run output sync.

Failure behavior:

- strict decision (`Decision.remote`) + handshake/submit failure => infra error
- fallback decision (`Decision.remote_any`) => try next node after terminal submit/preflight failure
- stream disconnect after submit => reconnect with `after_seq`
- duplicate submit with same `(task_run_id, attempt)` must attach to existing attempt, not create a new one

### 6.2 Transport Interface (Implementation Shape)

All protocol calls go through a single transport abstraction:

```text
trait Transport {
  capabilities() -> NodeCapabilitiesEnvelope
  status() -> NodeStatusEnvelope
  submit(req) -> SubmitAck
  events(task_run_id, after_seq) -> EventStream
  cancel(task_run_id) -> CancelAck
  result(task_run_id) -> TaskResultEnvelope
}
```

Normative: only `TransportFactory` may branch on transport variant.

### 6.3 Tor Transport Implementation (Arti, V1)

Tor mode (`RemoteTransportMode.TorOnionService`) is implemented in local `takd` as:

1. Parse config: onion endpoint, Arti config, restricted discovery material, service auth token.
2. Create/reuse Arti client instance scoped by node id.
3. Establish onion connection and run the same protocol contract as direct HTTPS.
4. Apply two auth layers:
   - transport-level Tor access control (restricted discovery/client authorization)
   - app-level service token for `takd` endpoint authorization.
5. Enforce timeouts/retries:
   - connection setup timeout
   - request timeout
   - stream idle timeout with resumable reconnect
6. Emit `transport_kind=tor` in placement/result records.

### 6.4 Tor Security and Operations (V1)

- node credentials are node-scoped; compromise of one node does not grant access to others
- support credential rotation with overlap windows (old+new accepted temporarily)
- redact tokens from logs and traces
- report Tor circuit/setup latency in transport metrics

## 7. Persistence and Local Visibility (V1)

For every task attempt, local run state must contain:

1. terminal status and exit code
2. full logs (streamed and persisted)
3. placement record:
   - mode (`local` or `remote`)
   - node id if remote
   - decision reason
4. synced outputs according to `OUTPUTS_AND_LOGS`
5. context record:
   - normalized `ContextManifest`
   - context manifest hash

## 8. Acceptance Criteria

1. `LocalOnly(Local(...))` executes locally and matches current local behavior.
2. `RemoteOnly(Remote(...))` executes remotely on that node or fails infra if unavailable.
3. `RemoteOnly([Remote(...), ...])` honors ordered fallback without scoring.
4. `ByCustomPolicy(policy_fn)` executes using only `PolicyContext` + policy result.
5. `CurrentState` controls transfer boundary via `roots -> ignored -> include`.
6. `include` re-includes ignored paths deterministically.
7. V1 accepts only `REPO_ZIP_SNAPSHOT` and `OUTPUTS_AND_LOGS`.
8. No numeric scoring terms appear in code, CLI, or docs.
9. Existing lease tests still pass unchanged.
10. Local/remote `takd` handshake follows preflight -> idempotent submit -> resumable events -> result flow.
11. Submit idempotency by `(task_run_id, attempt)` prevents duplicate remote execution.
12. Tor transport reaches onion `takd` through Arti with protocol parity to direct transport.
13. Remote auth failures produce explicit infra errors (no silent fallback unless `remote_any` is used).

## 9. Required Test Set (TDD Order)

1. Behavioral tests first:
   - `LocalOnly` contract
   - `RemoteOnly(single)` strict pin contract
   - `RemoteOnly(list)` ordered fallback contract
   - `ByCustomPolicy` contract (including reason visibility)
   - `CurrentState` transfer boundary contract (`roots -> ignored -> include`)
   - remote handshake contract (`preflight -> submit -> events -> result`)
   - stream resume contract (`after_seq` reconnect without duplication)
2. Unit tests:
   - policy IR parser/validator
   - reject unsupported decision helpers (builder API)
   - path anchor normalization and escape rejection
   - context manifest determinism + hash stability for identical inputs
   - transport factory dispatch + no variant branching outside factory
   - idempotency key and retry behavior for `(task_run_id, attempt)`
   - Tor transport config validation (endpoint/auth/arti settings)
   - constructor validation
3. Integration tests:
   - `tak` <-> local `takd` <-> remote `takd` flow
   - log streaming + result sync contract
   - remote payload contains only files from computed `ContextManifest`
   - direct and Tor transport parity on protocol behavior
   - auth failure and recovery paths (strict pin vs fallback list)
4. Mandatory gate:
   - `make check` must pass

## 10. Deferred (Post-V1)

- weighted or adaptive scoring
- telemetry staleness penalties
- additional transfer modes
- additional sync modes
- hedged execution
