# Tak Remote Execution Plan

**Status:** Final V1 spec (normative)

This document is the behavioral and type contract for adding remote execution to Tak. It is written to be implementable in Rust, with a Python DSL that provides **strong syntactic guarantees** via **Enums + dataclasses + sum types**.

## Non-negotiables (design decisions)

- **No stringly-typed configuration** for mode/policy knobs.
  - If it can be an enum, it **is** an enum.
  - If variants need different parameters, it **is** a sum type (Rust enum-with-data ↔ Python dataclasses).
  - Strings are used only for **values** (paths, labels, node IDs, human-readable reason codes, free-form tags).
- **Transport is an implementation detail**.
  - Transport is configured as a **typed sum type** and instantiated via a factory into a `Transport` interface.
  - The rest of the system is transport-neutral (Tor, Direct HTTPS, Nostr, etc. are adapters).
- **Logs are always available locally** (streamed live and persisted).
- **Artifacts are only synced if declared in `TASKS.py`** (and allowed by sync mode).

---

## 1. Vision

Enable `tak` to execute tasks locally or remotely with zero behavioral ambiguity:

- users keep writing tasks in `TASKS.py`
- placement (local vs remote) can be explicit or policy-driven
- remote communication uses a **transport-neutral protocol contract**
- a single run can leverage a pool of many remote `takd` nodes
- task outputs/logs/status return to local workspace in a deterministic way
- failures remain actionable and reproducible (including placement replay)

Remote execution must feel like a strict extension of local execution.

---

## 2. Core Goals (V1)

1. Add optional remote execution without breaking existing local workflows.
2. Keep task definitions elegant and expressive through a Python DSL with typed building blocks.
3. Make placement decisions explainable (`why local`, `why remote`, `why this node`).
4. **Local visibility contract (normative):**
   - Local Tak always records terminal status and exit code for every task.
   - Local Tak always streams and persists logs for every task (local or remote).
   - Declared outputs (artifacts) are synced back **only if declared** and permitted by `ResultSyncMode`.
5. Preserve reproducibility and security.
6. Progressive adoption: per-task, per-run, per-workspace.
7. Multi-node pools configured in Python workspace config.
8. Support at least one privacy-preserving transport adapter in V1 (Tor onion services) without requiring system daemons.

---

## 3. Non-Goals (V1)

1. No distributed DAG planner split across many clients (planner stays in local `tak`).
2. No arbitrary user Python execution on the server control plane.
3. No fully implicit side-effecting remote tasks by default.
4. No full mesh cross-node coordination in V1 (selection stays client-driven in `tak`).
5. We do not auto-solve every topology corner case; users get explicit, typed tools and can iterate.

---

## 4. Design Principles

1. **Deterministic by default (placement):**
   - Placement determinism is defined over a concrete, recorded input tuple:
     - `RunConfigSnapshot + TelemetrySnapshotAtDecisionTime + StableTieBreakKeyInputs`.
   - A placement decision must be replayable from persisted inputs (or a retrievable digest).
2. **Explainability over magic:**
   - Every placement decision emits a structured reason trace and stable `selection_trace_id`.
3. **Secure-by-default snapshots:**
   - Allowlist-first transfer boundary; denylist is additional defense.
4. **Composability:**
   - Use orthogonal enums/sum types; avoid giant configuration blobs.
5. **Graceful degradation:**
   - Clean local fallback when configured.
6. **Transport neutrality:**
   - Same protocol contract over any transport implementation.
7. **Multi-node first:**
   - Endpoint selection is explicit, observable, and configurable.

---

## 5. System Architecture

### 5.1 High-Level Components

1. **`tak` (local orchestrator)**
   - load/merge DAG
   - compute ready tasks
   - evaluate placement policy
   - submit remote task or run local
   - merge results into local run summary

2. **`takd` (remote control plane, server mode)**
   - runner registry and health
   - task queue and scheduling
   - cache lookup metadata
   - artifact index metadata
   - event stream hub (task lifecycle + log streaming)

3. **`takd` pool (N nodes)**
   - each node has independent capacity, queue pressure, and capabilities
   - `tak` selects node per task based on policy and telemetry snapshots
   - optional affinity/stickiness by run/group

4. **`tak-runner` (worker process on remote hosts)**
   - materialize workspace
   - execute task in isolated environment (process/container/VM)
   - stream logs and heartbeats
   - upload declared artifacts/results

5. **Artifact store**
   - filesystem (V1), abstracted for replaceable backends

6. **Metadata store**
   - sqlite (V1), abstracted for replaceable backends

### 5.2 Control/Data Separation

- Control plane: submit, cancel, status, scheduling, leases.
- Data plane: workspace transfer, log streaming, artifact upload/download.

### 5.3 Transport as an Adapter (Clean Architecture)

Transport is not a special “layer” in the architecture; it is an adapter that satisfies a `Transport` interface used by the remote client.

Normative: core scheduling/placement logic must not branch on transport type.

---

## 6. Execution Model Types (Rust ↔ Python)

### 6.1 Core Enums (finite choice knobs)

All enums are Rust `enum` ↔ Python `Enum` (1:1 names).

#### `ExecutionMode`
- `LOCAL_ONLY`
- `REMOTE_ONLY`
- `PREFER_LOCAL`
- `PREFER_REMOTE`
- `AUTO_ADAPTIVE`
- `HEDGED` *(V1 scope: deferred unless explicitly enabled in Section 19 mapping table)*

> REVIEW:
> `HEDGED` is a model-level value that can drift from user-visible behavior unless its CLI mapping is explicit.
> This plan keeps `HEDGED` in the type contract but marks it deferred in V1 by default.
> The canonical enum-to-CLI scope table is defined in Section 19.

Python DSL task syntax uses execution variants (sum type), not direct `ExecutionMode` literals:
- `execution=LocalOnly(...)`
- `execution=RemoteOnly(...)`
- `execution=PreferLocal(...)`
- `execution=PreferRemote(...)`
- `execution=AutoAdaptive(...)`
- `execution=Hedged(...)`

Loader normalization maps each variant to the corresponding `ExecutionMode` value in the structured IR.

#### `WorkspaceTransferMode`
- `REPO_ZIP_SNAPSHOT`
- `GIT_COMMIT_CLONE`
- `GIT_BUNDLE`
- `DELTA_PATCH`
- `DECLARED_INPUTS_ONLY`
- `PREBUILT_IMAGE`

#### `FallbackMode`
- `FAIL_CLOSED`
- `FAIL_OPEN_TO_LOCAL`
- `RETRY_THEN_LOCAL`
- `RETRY_REMOTE_ONLY`

#### `ResultSyncMode` (controls post-run payload sync; logs are still always present locally)
- `STATUS_ONLY` *(still streams/persists logs locally; only suppresses optional payload fetch)*
- `LOGS_ONLY` *(logs + minimal status envelope)*
- `OUTPUTS_ONLY` *(declared outputs + minimal status envelope; logs still local)*
- `OUTPUTS_AND_LOGS` *(recommended default)*
- `FULL_WORKDIR_DIFF` *(advanced, expensive)*

> REVIEW:
> The global invariant is "artifacts sync only if declared in `TASKS.py`."
> `FULL_WORKDIR_DIFF` is an explicit advanced exception and must be treated as opt-in and out of default V1 flows.
> When selected, CLI/help/runtime output must state that undeclared file changes may be transferred.

#### `IsolationMode`
- `PROCESS_SANDBOX`
- `CONTAINER_EPHEMERAL`
- `VM_EPHEMERAL`

#### `CachePolicy`
- `DISABLED`
- `READ_ONLY`
- `READ_WRITE`
- `WRITE_ONLY`

#### `CacheScope`
- `TASK`
- `RUN`
- `WORKSPACE`
- `PROJECT`

#### `SecretsPolicy`
- `NONE`
- `ALLOWLISTED`
- `SERVER_INJECTED_ONLY`

#### `NodeSelectionMode`
- `ROUND_ROBIN`
- `LEAST_LOADED`
- `CAPABILITY_AWARE`
- `AFFINITY`
- `ADAPTIVE_SCORE`

#### `NodeFailureMode`
- `TRY_NEXT_NODE`
- `RETRY_SAME_NODE`
- `FAIL_FAST`

#### `ConflictPolicy` (sync-back conflicts)
- `FAIL_ON_CONFLICT`
- `OVERWRITE`
- `KEEP_LOCAL_AND_STORE_REMOTE_COPY`

#### `NetworkPolicy`
- `DEFAULT`
- `NO_NETWORK`
- `EGRESS_LIMITED`
- `UNRESTRICTED`

#### `TelemetryStalenessPolicy`
- `STRICT_EXCLUDE_STALE`
- `ALLOW_STALE_WITH_PENALTY`

### 6.2 Platform & Capability Types (no string lists)

#### Enums
- `OperatingSystem`: `LINUX | MACOS | WINDOWS`
- `CpuArch`: `X86_64 | AARCH64`
- `ContainerRuntime`: `DOCKER | PODMAN | NONE`
- `VirtualizationSupport`: `NONE | VM_AVAILABLE`

#### Dataclass: `NodeCapabilities`
Rust struct ↔ Python dataclass:

- `os: OperatingSystem`
- `arch: CpuArch`
- `supported_isolation: frozenset[IsolationMode]`
- `container_runtime: ContainerRuntime`
- `virtualization: VirtualizationSupport`
- `tags: frozenset[str]` *(free-form values)*

**Loader rule (normative):** capability string lists (e.g. `["linux", "aarch64", ...]`) are invalid.

### 6.3 Transport Configuration (sum type, not “enum + endpoint”)

#### 6.3.1 Rust: enum-with-data (normative)

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RemoteTransportMode {
    DirectHttps(DirectHttpsConfig),
    TorOnionService(TorOnionServiceConfig),
    // reserved:
    // Nostr(NostrConfig),
}
```

#### 6.3.2 Python: dataclasses mirroring variants (normative)

```python
class RemoteTransportMode:
    @dataclass(frozen=True)
    class DirectHttps:
        endpoint: str               # "https://…"
        tls: TlsClientConfig | None = None
        auth: ServiceAuth | None = None

    @dataclass(frozen=True)
    class TorOnionService:
        endpoint: str               # "http://….onion" (or onion://…)
        arti: ArtiClientConfig | None = None
        restricted_discovery: RestrictedDiscovery | None = None
        auth: ServiceAuth | None = None
```

**Invalid by design:** `transport=RemoteTransportMode.DIRECT_HTTPS, endpoint="..."`

#### 6.3.3 Clean Architecture: `Transport` interface + factory

- `RemoteTransportMode` is configuration, not the implementation.
- Runtime uses:

```text
TransportFactory(RemoteTransportMode) -> Transport
RemoteClient(transport: Transport)
```

Normative: code outside `TransportFactory` must not branch on transport variant.

### 6.4 Task Context & Path Addressing (normative)

#### Path anchors

Every task path resolves from an explicit anchor:

- `PACKAGE` — directory containing the declaring `TASKS.py`
- `WORKSPACE` — detected Tak workspace root
- `REPO` — named repository root configured in workspace config

Named repository roots are declared in workspace config as typed entries:

- `RepoRootSpec { id: str, root: str }`

Python DSL path forms:

- `path("services/api/**")` -> `PACKAGE`
- `path("//services/api/**")` -> `WORKSPACE`
- `path("@infra//docker/base.Dockerfile")` -> `REPO(id="infra")`

#### Normalization and safety

- normalize separators to `/`
- collapse `.` segments
- reject escapes outside anchor (`..` crossing anchor root)
- reject host-absolute paths by default (advanced opt-in only)
- preserve glob tokens (`*`, `**`, `?`, character classes)

Resolved IR stores typed data, not raw strings:

- `PathRef { anchor, repo_id?, rel }`

#### Task context declaration

V1 task context uses a single typed class:

- `context=CurrentState(...)`

`CurrentState` fields:

- `roots: list[path]` (default `[path("//")]`)
- `ignored: list[IgnoreSource]` (default `[]`)
- `include: list[path]` (default `[]`, applied after ignores)

`IgnoreSource` variants:

- `path("glob-or-path")`
- `gitignore()` *(optional source; if no gitignore file is present, source contributes nothing)*

Normalization and precedence:

- nested ignore lists are flattened before normalization
- flatten and dedupe all ignore sources into a unified ignore list
- base transfer set comes from `roots`
- apply unified ignores
- apply `include` as explicit re-inclusions (override ignores)
- reject `include` paths outside selected roots

Resulting transfer set is persisted as `ContextManifest` and hashed for cache/explain.

---

## 7. Python DSL Design

### 7.1 Philosophy

1. Valid Python, no string mini-language.
2. Typed building blocks (enums + dataclasses + sum types).
3. Rust receives structured IR with explicit tags.

### 7.2 Minimal example (no strings-as-modes)

```python
from tak import task, cmd, results, constraints, path
from tak.remote import (
    PreferRemote,
    WorkspaceTransferMode,
    ResultSyncMode,
    ConflictPolicy,
    NetworkPolicy,
)

task(
    ":test",
    steps=[cmd("pytest", "-q")],
    outputs=[path("out/test-report.xml")],
    execution=PreferRemote(
        workspace=WorkspaceTransferMode.REPO_ZIP_SNAPSHOT,
        results=results(sync=ResultSyncMode.OUTPUTS_AND_LOGS, conflict=ConflictPolicy.FAIL_ON_CONFLICT),
        constraints=constraints(network=NetworkPolicy.DEFAULT),
    ),
)
```

### 7.3 Workspace config example (typed nodes, typed transport, typed capabilities)

```python
from tak import workspace_config, module_spec
from tak.remote import (
    ExecutionMode, WorkspaceTransferMode, ResultSyncMode,
    TelemetryStalenessPolicy,
    NodeSelectionMode, NodeFailureMode,
    OperatingSystem, CpuArch, ContainerRuntime, VirtualizationSupport,
    IsolationMode, NodeCapabilities, RemoteTransportMode,
    CachePolicy, CacheScope,
    remote_defaults, selection_weights, remote_node,
    ServiceAuth, RestrictedDiscovery,
    merge_nodes, env_nodes,
)

WORKSPACE = workspace_config(
    project_id="example_project",
    remote=remote_defaults(
        enabled=True,
        default_mode=ExecutionMode.AUTO_ADAPTIVE,
        workspace_transfer=WorkspaceTransferMode.DELTA_PATCH,
        result_sync=ResultSyncMode.OUTPUTS_AND_LOGS,
        node_selection=NodeSelectionMode.ADAPTIVE_SCORE,
        node_failure_mode=NodeFailureMode.TRY_NEXT_NODE,
        telemetry_ttl_ms=3000,
        telemetry_staleness=TelemetryStalenessPolicy.STRICT_EXCLUDE_STALE,
        cache_policy=CachePolicy.READ_WRITE,
        cache_scope=CacheScope.PROJECT,
        selection_weights=selection_weights(
            queue_eta=0.35,
            capacity=0.30,
            cache_hit_probability=0.20,
            transfer_cost=0.15,
        ),
        nodes=merge_nodes(
            [
                remote_node(
                    id="build-us-x86",
                    transport=RemoteTransportMode.TorOnionService(
                        endpoint="http://abc123def456.onion",
                        auth=ServiceAuth.from_env("TAK_NODE_BUILD_US_X86_TOKEN"),
                        restricted_discovery=RestrictedDiscovery.from_env("TAK_NODE_BUILD_US_X86_RD_KEY"),
                    ),
                    capabilities=NodeCapabilities(
                        os=OperatingSystem.LINUX,
                        arch=CpuArch.X86_64,
                        supported_isolation=frozenset({IsolationMode.CONTAINER_EPHEMERAL}),
                        container_runtime=ContainerRuntime.DOCKER,
                        virtualization=VirtualizationSupport.NONE,
                        tags=frozenset({"ssd-cache"}),
                    ),
                    weight=1.0,
                    max_parallel_tasks=8,
                ),
            ],
            env_nodes("TAK_REMOTE_NODES_JSON"),
        ),
    ),
)

SPEC = module_spec(tasks=[...], workspace=WORKSPACE)
SPEC
```

Named repository roots used by `path("@repo//...")` are configured in workspace config and snapshotted at run start.

### 7.4 Placement policy functions (deterministic IR)

Canonical decision API (must match docs/examples):

- `Decision.start()`
- `Decision.local(reason=...)`
- `Decision.remote(reason=...)`
- `decision.prefer_remote(weight, reason=...)`
- `decision.prefer_local(weight, reason=...)`
- `decision.prefer_node(node_id, weight, reason=...)`
- `decision.forbid_mode(mode, reason=...)`

Allowed AST subset (V1):
- `if/elif/else`, boolean ops, comparisons
- literals
- attribute access on `state`
- `return`
- calls to the above decision methods

Disallowed:
- loops
- I/O (file/network/process)
- randomness/time dependence
- imports inside policy function

### 7.5 Context and path example (anchored, deterministic)

```python
from tak import task, cmd, path, gitignore
from tak.remote import RemoteOnly, WorkspaceTransferMode, CurrentState

task(
    ":api_tests",
    steps=[cmd("pytest", "-q", cwd="services/api")],
    context=CurrentState(
        roots=[
            path("//"),                              # workspace-relative root
            path("@infra//"),                        # named repo root
        ],
        ignored=[
            path("**/.venv/**"),
            path("**/node_modules/**"),
            gitignore(),
        ],
        include=[
            path("//node_modules/.bin/eslint"),      # overrides ignored
        ],
    ),
    execution=RemoteOnly(workspace=WorkspaceTransferMode.DELTA_PATCH),
)
```

Loader outputs a stable `ContextManifest` per task:
- normalized `PathRef[]`
- unified ignore list
- include override list
- digest list + manifest hash used by cache and explain output

---

## 8. Placement Engine

### 8.1 Decision pipeline

1. Apply hard constraints (must local / must remote).
2. Apply execution mode.
3. For `AUTO_ADAPTIVE`:
   - base heuristics (Rust)
   - user policy IR adjustments
4. If remote chosen:
   - filter nodes by typed capabilities and health
   - evaluate telemetry freshness
   - compute score per node
   - deterministic tie-break
5. Emit and persist:
   - `PlacementDecision`
   - `selection_trace_id` referencing a persisted decision packet

### 8.2 Determinism input packet (must be persisted or retrievable)

Per task decision, store either the full packet or a digest + retrievable record:

- `RunConfigSnapshot` (normalized + hash)
- `TaskIdentity` (`task_label`, `task_run_id`, attempt)
- `CandidateNodes` (ordered after filtering)
- `TelemetrySnapshotAtDecisionTime` (including timestamps/ages)
- scoring weights + versions
- tie-break key inputs

### 8.3 Telemetry caching and freshness (normative thresholds)

Let `ttl_ms = telemetry_ttl_ms`.

- `FRESH`: `age <= ttl_ms` → eligible, no penalty
- `STALE`: `ttl_ms < age <= 3*ttl_ms` → eligible with defined penalty (recorded)
- `INELIGIBLE`: `age > 3*ttl_ms` or missing → excluded unless `TelemetryStalenessPolicy.ALLOW_STALE_WITH_PENALTY` is configured

Placement trace must record:
- telemetry age + freshness state for chosen node
- penalty/exclusion reasons used

> REVIEW:
> Run snapshots are immutable per run, but telemetry is intentionally time-varying.
> Determinism is defined over the persisted decision packet (`RunConfigSnapshot + TelemetrySnapshotAtDecisionTime + tie-break inputs`), not over run config alone.
> Replay must resolve to the same placement when fed the persisted packet.

### 8.4 Deterministic tie-break (normative)

Final tie-break key for candidate node `i`:

- `tie_i = blake3( snapshot_hash || task_run_id || candidate_node_id || scoring_version )`
- order lexicographically by `tie_i` as last key

### 8.5 Explainability contract

Every decision must be inspectable:
- selected mode (local/remote)
- node selected (if remote)
- factor scores and weights
- constraints triggered
- fallback path (if any)
- telemetry freshness signals

CLI:
- `tak run ... --explain-placement`
- `tak explain <label> --placement`

---

## 9. Workspace Materialization

### 9.1 Transfer modes
- `REPO_ZIP_SNAPSHOT`: archive snapshot + manifest + checksums
- `GIT_COMMIT_CLONE`: clone + checkout
- `GIT_BUNDLE`: ship bundle
- `DELTA_PATCH`: send only changes atop base
- `DECLARED_INPUTS_ONLY`: send only the context-filtered transfer set from `CurrentState`
- `PREBUILT_IMAGE`: run with OCI image + optional mounts

For `CurrentState`, transfer set is computed from `ContextManifest` (`roots -> ignored -> include`) and must be identical across retries for the same run snapshot.

### 9.2 Secret safety (allowlist-first)

- denylist patterns exist but are not sufficient alone
- for modes that ship workspace files, require allowlist boundary:
  - explicit include globs, or
  - explicit `CurrentState` roots/ignored/include filters
- fail closed on high-confidence secret matches unless explicitly overridden and recorded

### 9.3 Context manifest and repository roots

- named repo roots are resolved at run start into `RunConfigSnapshot`
- `path("@repo//...")` must resolve to a configured, existing repo root
- context manifest records resolved roots by stable repo id and digest
- ignore/include sources are normalized and persisted in manifest order
- repo root changes require a new run snapshot (or explicit refresh)

---

## 10. Remote Protocol (transport-neutral)

### 10.1 Required endpoints (canonical HTTP contract)

Even when carried over non-HTTP transports, semantics are identical.

1. `POST /v1/tasks/submit` (idempotent; key = `(task_run_id, attempt)`)
2. `GET /v1/tasks/{task_run_id}/events` (canonical framing: NDJSON)
3. `POST /v1/tasks/{task_run_id}/cancel`
4. `GET /v1/tasks/{task_run_id}/result`
5. `GET /v1/node/status` (telemetry snapshot)
6. `GET /v1/node/capabilities` (typed capabilities)

### 10.2 Events stream (NDJSON, ordered, resumable)

Each event includes:
- `seq` (monotonic int)
- `task_run_id`
- `type` (enum)
- `timestamp_ms`
- `payload` (typed per event)

Resume:
- `?after_seq=<n>`
- server guarantees no duplication past `after_seq`
- server defines retention window for replay; if expired, returns a marker event indicating discontinuity

### 10.3 Event types (canonical V1 set)

- `TASK_QUEUED`
- `TASK_ASSIGNED`
- `WORKSPACE_MATERIALIZING`
- `TASK_STARTED`
- `TASK_LOG_CHUNK`
- `TASK_HEARTBEAT`
- `TASK_RETRYING`
- `TASK_FINISHED`
- `TASK_FAILED`
- `TASK_CANCELED`
- `ARTIFACT_UPLOADED`

### 10.4 Result envelope (must return locally)

Required fields:
- `task_label`
- `task_run_id`
- `attempt`
- `status` (`success | failed | timed_out | canceled | infra_error`)
- `exit_code`
- `started_at`, `finished_at`, `duration_ms`
- `runner_id`, `host`, `isolation_mode`
- `node_id`, `transport_kind`
- `log_artifact_uri`
- `output_artifacts[]` (`path`, `digest`, `size`, `uri`)
- `stdout_tail`, `stderr_tail` (bounded)
- `failure_reason` (structured, optional)

Local Tak must merge this envelope into local run summary and output contracts.

### 10.5 Multi-node client contract

- `tak` polls/subscribes node telemetry with local TTL caching.
- stale nodes are penalized/excluded per `TelemetryStalenessPolicy`.
- each submit includes:
  - selected `node_id`
  - `selection_trace_id`
  - placement rationale fingerprint (debug/audit)

> REVIEW:
> Submit idempotency and retry identity must be unambiguous.
> V1 contract: one logical task has stable `task_run_id`; each retry increments `attempt`.
> `(task_run_id, attempt)` is unique; submission idempotency key is the tuple.

---

## 11. Results, Logs, and Artifact Sync (local)

### 11.1 Contract (normative)

Local `.tak/runs/<run_id>/` contains:
- per-task result JSON (status, exit code, timings)
- per-task log file (always)
- placement decision JSON (when adaptive)
- outputs manifest and completeness markers (when outputs enabled)

### 11.2 Set-level output commit (normative)

- stage all outputs
- verify all digests
- commit with manifest marker `commit.ok`
- on failure: rollback staged outputs and write `commit.failed`

### 11.3 Conflict policy

`ConflictPolicy` controls what happens if local files changed while remote ran.

- `FAIL_ON_CONFLICT` (default)
- `OVERWRITE`
- `KEEP_LOCAL_AND_STORE_REMOTE_COPY`

---

## 12. Configuration Snapshot & Precedence

Run-start snapshot:
- resolve dynamic sources once at run start into `RunConfigSnapshot`
- placement decisions use the run snapshot plus decision-time telemetry packet
- snapshot hash stored for replay
- discovery refresh requires explicit user action (`--refresh-remote-config`) or a new run

Precedence:
`CLI > env > workspace config > defaults`

Env var naming canonicalization:
- `TAK_REMOTE_MODE`
- `TAK_REMOTE_NODE`
- `TAK_REMOTE_NODES_JSON`
- `TAK_NODE_<NODE_ID_NORM>_TOKEN`
- `TAK_NODE_<NODE_ID_NORM>_RD_KEY`
where `NODE_ID_NORM = upper(node_id).replace('-', '_')` and invalid characters are rejected.

---

## 13. Acceptance Criteria

1. `tak run <label> --remote auto` shows clear placement lines.
2. Logs always stream and persist locally for remote tasks.
3. Declared output artifacts sync back with checksum verification and completeness marker (except explicit `FULL_WORKDIR_DIFF` mode).
4. Fallback behavior is followed and explained.
5. Placement decisions can be replayed from persisted inputs.
6. E2E matrix covers in-scope transfer/fallback/sync modes.
7. Snapshot security prevents accidental secret transfer by default.
8. Pool with ≥3 nodes distributes work by telemetry snapshots.
9. Per-task output includes node id and transport kind.
10. Node failure handling is observable and tested.
11. `CurrentState` include paths override ignored paths deterministically and are visible in explain output.

---

## 14. Scheduler and Runner Strategy

### 14.1 Scheduler Inputs

- task resource hints (`cpu`, `ram`, `gpu`, `io`)
- runner capabilities and current usage
- queue priority and SLA class
- data locality / transfer cost estimates

### 14.2 Scheduling policies

- `FIFO`
- `PRIORITY`
- `SHORTEST_ESTIMATED_FIRST`
- `FAIR_SHARE` (per user/project)
- `WEIGHTED_ADAPTIVE` (node-aware weighting across remote pool)

### 14.3 Runner lifecycle

1. register
2. heartbeat
3. accept assignment
4. execute
5. upload results
6. ack completion
7. become available

### 14.4 Retries (normative)

Differentiate:
- task retries (user policy)
- infra retries (transient network/runner/transport failures)

Rules:
- infra retries must not hide deterministic task failures.
- every retry increments `attempt`.
- explain output must include retry class (`task` vs `infra`) and reason.

### 14.5 Cross-node distribution (client-driven in V1)

For each ready task:
1. filter nodes by compatibility (`os`, `arch`, isolation, features)
2. filter nodes by health/freshness
3. score per node (`queue_eta`, available slots, failure rate, transfer estimate, policy IR adjustments)
4. choose best node + deterministic tie-break
5. on infra failure, apply `NodeFailureMode`

---

## 15. Caching Model

### 15.1 Remote cache key

Digest should include:
- normalized task label
- step commands/scripts
- declared dependency version digests
- declared input digests
- env allowlist values
- toolchain image/runtime fingerprint
- relevant DSL execution config
- normalized context manifest hash (anchored paths + artifact refs)

### 15.2 Cache safety

- do not cache side-effecting tasks by default
- namespace cache by project/workspace
- explicit invalidation command

### 15.3 Cache UX

- `tak run ... --cache=off|read|rw`
- verbose mode prints cache hit/miss reason

---

## 16. Security Model

### 16.1 AuthN/AuthZ

- direct transport supports mTLS or signed service tokens
- Tor transport supports Restricted Discovery + service token
- user token for run attribution
- RBAC for submit/cancel/read-logs/artifacts
- per-node credentials in workspace config (no global shared secret)

### 16.2 Tor-specific access control

- each `takd` onion service has independent service identity and client authorization material
- compromised credentials for one node must not grant access to others
- pairing material must be node-scoped, rotatable, revocable

### 16.3 Secret handling

- prefer server-injected secrets where possible
- do not serialize secrets into workspace snapshots
- redact secrets in logs (best effort + known patterns)

### 16.4 Isolation and hardening

- run tasks as unprivileged user
- enforce CPU/memory/fd limits
- apply per-task network policy
- guarantee ephemeral workspace cleanup

### 16.5 Key/token rotation

- rotate service auth and discovery credentials on schedule
- support dual-key grace windows to avoid downtime
- expose rotation status via remote status surfaces

---

## 17. Observability and Operations

### 17.1 Metrics

- queue depth
- scheduler latency
- task wait/run duration
- placement distribution (`local` vs `remote`)
- placement distribution per node
- fallback rate
- cache hit rate
- artifact transfer throughput/failure
- transport setup latency (including Tor circuit setup)
- node health and telemetry staleness rates

### 17.2 Logs and tracing

- structured logs with `run_id`, `task_label`, `task_run_id`, `node_id`
- trace spans across `tak -> takd -> tak-runner`

### 17.3 Operational commands

- `tak remote status`
- `tak remote nodes`
- `tak remote runners`
- `tak remote runs --recent`
- `tak remote logs <task_run_id>`
- `tak remote pair <node_id>`
- `tak remote verify-node <node_id>`

---

## 18. Failure Model and Recovery

### 18.1 Failure categories

1. planning/config errors
2. transfer/materialization errors
3. scheduler capacity errors
4. runner infra errors
5. task command failures
6. sync-back conflicts/errors

### 18.2 Recovery strategy

- resumable uploads/downloads
- idempotent submission with deterministic request identity
- heartbeat timeout -> task marked lost -> retry policy applied
- partial artifact upload cleanup via TTL sweeper

### 18.3 User-facing failure contract

For each failure report:
- category
- root cause summary
- fallback attempt status
- next recommended action

---

## 19. CLI and UX Contract

### 19.1 Canonical enum-to-CLI mapping (normative)

`ExecutionMode`:
- `LOCAL_ONLY` -> `--remote off`
- `REMOTE_ONLY` -> `--remote required`
- `PREFER_LOCAL` -> `--remote prefer-local`
- `PREFER_REMOTE` -> `--remote prefer-remote`
- `AUTO_ADAPTIVE` -> `--remote auto`
- `HEDGED` -> `--remote hedged` (deferred in V1 unless explicitly enabled)

`WorkspaceTransferMode`:
- `REPO_ZIP_SNAPSHOT` -> `--remote-transfer zip`
- `GIT_COMMIT_CLONE` -> `--remote-transfer git-clone`
- `GIT_BUNDLE` -> `--remote-transfer bundle`
- `DELTA_PATCH` -> `--remote-transfer delta-patch`
- `DECLARED_INPUTS_ONLY` -> `--remote-transfer inputs-only`
- `PREBUILT_IMAGE` -> `--remote-transfer prebuilt-image` (advanced; may be hidden in V1)

`FallbackMode`:
- `FAIL_CLOSED` -> `--remote-fallback fail-closed`
- `FAIL_OPEN_TO_LOCAL` -> `--remote-fallback fail-open-local`
- `RETRY_THEN_LOCAL` -> `--remote-fallback retry-then-local`
- `RETRY_REMOTE_ONLY` -> `--remote-fallback retry-remote-only`

`ResultSyncMode`:
- `STATUS_ONLY` -> `--sync status-only`
- `LOGS_ONLY` -> `--sync logs-only`
- `OUTPUTS_ONLY` -> `--sync outputs-only`
- `OUTPUTS_AND_LOGS` -> `--sync outputs-and-logs`
- `FULL_WORKDIR_DIFF` -> `--sync full-workdir-diff`

Other key flags:
- `--remote-transport direct|tor|auto`
- `--remote-node <node_id>`
- `--remote-node-policy round-robin|least-loaded|capability-aware|affinity|adaptive-score`
- `--ignore <glob-or-path>` *(repeatable; adds runtime ignore sources)*
- `--include <path>` *(repeatable; runtime include override, applied after ignores)*
- `--explain-placement`
- `--refresh-remote-config`

> REVIEW:
> This table is the single source of truth for CLI token stability and model parity.
> If a token is hidden/deferred, that status must be explicit here and in `--help`.
> Parser tests must cover every row.

### 19.2 Suggested defaults (safe + useful)

- global remote default: `off` (initial rollout)
- per-project opt-in default: `auto`
- transfer default (when remote enabled): `DELTA_PATCH` for inner-loop speed
- sync default: `OUTPUTS_AND_LOGS`
- fallback default: `RETRY_THEN_LOCAL`

Override triggers for reproducibility-sensitive workflows:
- release pipelines
- incident replay
- deterministic debugging sessions

Recommended override behavior:
- force transfer to `REPO_ZIP_SNAPSHOT`
- keep sync at `OUTPUTS_AND_LOGS` unless explicitly narrowed
- persist full placement explain packet

### 19.3 UX contract requirements

- clear per-task status line with placement and reason code
- live streaming logs regardless of local/remote execution
- single-command experience (`tak run`)

Expected line shape:

```text
//apps/api:build placement=remote(node=build-us-x86, transport=tor, reason=local_cpu_high+queue_ok) status=ok attempts=1
//apps/api:test  placement=local(mode=forced, reason=side_effecting)                               status=ok attempts=1
```

Failover shape:

```text
//apps/api:test placement=remote(node=build-us-x86) status=infra_error failover=build-eu-arm64 status=ok attempts=2
```

---

## 20. Crate-by-Crate Implementation Plan

### 20.1 `crates/tak-core`

Add model primitives:
- enums from Section 6
- `TaskExecutionSpec`
- `NodePoolSpec` and `RemoteNodeSpec`
- `PlacementDecision`
- `TaskResultEnvelope`
- strict serde contracts + validation

### 20.2 `crates/tak-loader`

- extend Python-to-JSON conversion for new DSL primitives
- validate incompatible combinations early
- canonicalize defaults and inheritance rules

### 20.3 `crates/tak-exec`

- add placement engine
- add remote client abstraction
- implement local/remote branch with shared summary contract
- integrate sync-back pipeline and conflict policy
- add node selection and failover behavior
- add Tor transport adapter via Arti

### 20.4 `crates/takd`

Preferred V1:
- extend existing `takd` with server mode
- runner registry
- scheduler queue
- task state machine
- artifact metadata hooks
- optional embedded onion endpoint

### 20.5 `crates/tak`

- implement CLI flags and docs
- placement explanations in output
- remote status subcommands
- node pool visibility commands
- pairing and credential helpers

### 20.6 New crate candidates

- `tak-remote-proto` (shared API models)
- `tak-runner` (worker executable)
- `tak-artifacts` (storage abstraction)
- `tak-tor` (transport abstraction over Arti)

---

## 21. Incremental Delivery Phases

### Phase 0: Contracts

- finalize enums and DSL schema
- finalize protocol and result envelope
- finalize compatibility guarantees

### Phase 1: Remote skeleton

- `REMOTE_ONLY` + `PREFER_REMOTE`
- `REPO_ZIP_SNAPSHOT`
- `OUTPUTS_AND_LOGS`
- single runner + single server
- direct transport first

### Phase 2: Reliability

- fallback modes
- retries/idempotency
- conflict policies
- multi-node config + basic selection

### Phase 3: Adaptive placement

- `AUTO_ADAPTIVE`
- telemetry scoring
- `--explain-placement`
- Tor transport support

### Phase 4: Advanced transfer + cache

- `DELTA_PATCH`, `GIT_BUNDLE`
- remote cache read/write
- cache observability
- discovery credential lifecycle + rotation

### Phase 5: Hardening + scale

- multi-runner pools
- stronger isolation options
- RBAC and audit trails
- optional federated coordination

---

## 22. Testing and Quality Plan

### 22.1 Required TDD order (repo policy)

Execution order is normative:
1. BDD/behavioral tests first (including UX contract tests for user-visible output)
2. unit tests
3. integration tests
4. implementation
5. refactor after green

### 22.2 Unit tests

- enum parsing/validation
- anchored path parsing/normalization and escape rejection
- `CurrentState` ignore/include merge precedence and determinism
- placement scoring determinism
- transfer mode selection
- conflict resolution logic
- precedence matrix (`CLI > env > workspace > defaults`)
- snapshot canonicalization/hash stability
- snapshot immutability after run start

### 22.3 Integration tests

- local `tak` <-> remote `takd` protocol contract
- runner lifecycle transitions
- artifact roundtrip + checksum verification
- fallback behavior under induced failures
- direct vs Tor transport parity
- multi-node selection/failover
- explanation completeness under stale telemetry and failover
- `CurrentState` transfer boundary enforcement (`roots -> ignored -> include`) from context manifest

### 22.4 End-to-end matrix

Axes:
- execution mode
- transfer mode
- fallback mode
- sync mode
- with/without cache
- transport mode
- node selection mode

### 22.5 Chaos/resilience tests

- kill runner mid-task
- network partition during logs/artifact upload
- corrupted snapshot/archive
- stale queue entries

### 22.6 UX contract tests

- output line format stability
- placement explain output stability
- failure messaging per category

### 22.7 Mandatory validation commands

- run `make check` for every change before marking work complete
- completion report must include command and pass/fail result

> REVIEW:
> Prior distilled acceptance criteria required E2E coverage but did not encode mandatory TDD ordering and `make check` gates.
> This section restores those repo-level contracts as normative.

---

## 23. Documentation Plan

1. add `docs/remote-execution/overview.md`
2. add `docs/remote-execution/dsl.md`
3. add `docs/remote-execution/operations.md`
4. add `docs/remote-execution/security.md`
5. add `docs/remote-execution/tor.md`
6. add `docs/remote-execution/node-pools.md`
7. include doctest-style parser/CLI examples where applicable

---

## 24. Configuration Plan (Python-first)

### 24.1 Workspace config in `TASKS.py`

Workspace config can define:
- remote defaults and placement strategy
- transport strategy
- node pool inventory
- per-node auth and capabilities
- artifact/cache backend config

### 24.2 Environment overrides

Supported overrides (V1):
- `TAK_REMOTE_MODE`
- `TAK_REMOTE_NODE`
- `TAK_REMOTE_NODES_JSON`
- `TAK_NODE_<NODE_ID_NORM>_TOKEN`
- `TAK_NODE_<NODE_ID_NORM>_RD_KEY`

All values must parse into typed enums/sum-types; invalid values are hard errors.

### 24.3 Snapshot and precedence

Reference Sections 12 and 8:
- dynamic sources resolved at run start into `RunConfigSnapshot`
- placement uses run snapshot + decision-time telemetry packet
- precedence remains `CLI > env > workspace > defaults`

---

## 25. Data Retention and Compliance

- retention policy for logs/artifacts (per project)
- PII/secrets redaction policy
- audit-event retention window
- deletion/erase hooks when required by policy

---

## 26. Open Questions (to settle before implementation freeze)

1. keep `takd` single binary with modes, or split control plane later?
2. which artifact backend is mandatory in V1 (local FS only or S3-compatible)?
3. how strict is output declaration for all remote tasks in V1?
4. keep `HEDGED` deferred or enable behind explicit experimental flag?
5. allow nested remote task invocation or require leaf command execution only?
6. should Tor be mandatory, or one first-class option among transports?
7. what is the recommended Restricted Discovery distribution model at scale?
8. should cross-node fairness remain purely client-driven in V1?

---

## 27. Recommended V1 decisions

1. keep one `takd` binary with explicit run modes
2. prioritize `REPO_ZIP_SNAPSHOT` and `DELTA_PATCH`
3. enforce declared outputs for remote tasks by default
4. require `side_effecting=False` for `AUTO_ADAPTIVE` in V1
5. default fallback to `RETRY_THEN_LOCAL`
6. support Tor fully in V1 while keeping direct transport optional
7. keep node selection client-driven (`ADAPTIVE_SCORE` default)
8. use per-node credentials with independent revocation

---

## 28. Example End-to-End User Experience

```bash
tak run //apps/api:test \
  --remote auto \
  --remote-transport tor \
  --remote-node-policy adaptive-score \
  --explain-placement
```

Expected per-task lines:

```text
//apps/api:build placement=remote(node=build-us-x86, transport=tor, reason=local_cpu_high+queue_ok) status=ok attempts=1
//apps/api:test  placement=remote(node=build-eu-arm64, transport=tor, reason=cache_hit+arm64_idle) status=ok attempts=1
```

---

## 29. Glossary (plain language)

- `artifact`: file produced by a task (for example report, binary, log)
- `attempt`: one run of a task; retries create additional attempts
- `AUTO_ADAPTIVE`: mode that scores local vs remote per task
- `cache hit`: requested result found in cache
- `cache miss`: result not in cache; task must execute
- `deterministic tie-break`: final stable ordering key when scores are equal
- `idempotent submit`: repeated submit with same identity does not duplicate work
- `placement decision`: selected execution mode and node (if remote), with reasons
- `Restricted Discovery`: Tor access-control mechanism for onion services
- `RunConfigSnapshot`: resolved run config frozen at run start
- `selection_trace_id`: stable identifier for persisted placement explanation packet
