from typing import Literal
from typing_extensions import TypedDict


# One explicit workspace path selected with `path(...)`.
class PathSelector(TypedDict):
    kind: Literal["path"]
    value: str


# One explicit output glob selected with `glob(...)`.
class GlobOutput(TypedDict):
    kind: Literal["glob"]
    value: str


# Reuse the repo gitignore rules as a CurrentState ignore source.
class GitignoreSource(TypedDict):
    kind: Literal["gitignore"]


# One bind mount entry for a container runtime.
class ContainerMountSpec(TypedDict):
    source: str
    target: str
    read_only: bool


# Optional CPU and memory limits for a container runtime.
class ContainerResourceLimitsSpec(TypedDict, total=False):
    cpu_cores: float
    memory_mb: int


# Remote container runtime built from a prebuilt image.
class ContainerRuntimeSpec(TypedDict):
    kind: Literal["containerized"]
    image: str
    dockerfile: None
    build_context: None
    command: list[str] | None
    mounts: list[ContainerMountSpec]
    env: dict[str, str]
    resource_limits: ContainerResourceLimitsSpec | None


# Remote container runtime built from a workspace Dockerfile path.
class DockerfileRuntimeSpec(TypedDict):
    kind: Literal["containerized"]
    image: None
    dockerfile: PathSelector
    build_context: PathSelector | None
    command: list[str] | None
    mounts: list[ContainerMountSpec]
    env: dict[str, str]
    resource_limits: ContainerResourceLimitsSpec | None


# Direct HTTPS remote transport selector.
class DirectHttpsTransportSpec(TypedDict):
    kind: Literal["direct"]


# Transport selector that lets Tak choose direct or Tor.
class AnyTransportSpec(TypedDict):
    kind: Literal["any"]


# Tor onion-service remote transport selector.
class TorOnionServiceTransportSpec(TypedDict):
    kind: Literal["tor"]


# Local execution pool returned by `Local(...)`.
class LocalSpec(TypedDict):
    id: str
    max_parallel_tasks: int
    runtime: ContainerRuntimeSpec | DockerfileRuntimeSpec | None


# Remote execution target returned by `Remote(...)`.
class RemoteSpec(TypedDict):
    pool: str | None
    required_tags: list[str]
    required_capabilities: list[str]
    transport: DirectHttpsTransportSpec | AnyTransportSpec | TorOnionServiceTransportSpec | None
    runtime: ContainerRuntimeSpec | DockerfileRuntimeSpec | None


# Name plus scope reference reused by needs and queue usage.
class LimiterRef(TypedDict):
    name: str
    scope: Literal["machine", "user", "project", "worktree"]


# Limiter lease request returned by `need(...)`.
class NeedSpec(TypedDict):
    limiter: LimiterRef
    slots: float
    hold: Literal["during", "at_start"]


# Queue lease request returned by `queue_use(...)`.
class QueueUseSpec(TypedDict):
    queue: LimiterRef
    slots: int
    priority: int


# Capacity-based limiter returned by `resource(...)`.
class ResourceLimiterSpec(TypedDict):
    kind: Literal["resource"]
    name: str
    scope: Literal["machine", "user", "project", "worktree"]
    capacity: float
    unit: str | None


# Exclusive limiter returned by `lock(...)`.
class LockLimiterSpec(TypedDict):
    kind: Literal["lock"]
    name: str
    scope: Literal["machine", "user", "project", "worktree"]


# Token-bucket limiter returned by `rate_limit(...)`.
class RateLimitLimiterSpec(TypedDict):
    kind: Literal["rate_limit"]
    name: str
    scope: Literal["machine", "user", "project", "worktree"]
    burst: int
    refill_per_second: float


# External-process cap returned by `process_cap(...)`.
class ProcessCapLimiterSpec(TypedDict):
    kind: Literal["process_cap"]
    name: str
    scope: Literal["machine", "user", "project", "worktree"]
    max_running: int
    match: str | None


# Queue definition returned by `queue_def(...)`.
class QueueDefinition(TypedDict):
    name: str
    scope: Literal["machine", "user", "project", "worktree"]
    slots: int
    discipline: Literal["fifo", "priority"]
    max_pending: int | None


# Fixed retry backoff returned by `fixed(...)`.
class FixedBackoffSpec(TypedDict):
    kind: Literal["fixed"]
    seconds: float


# Exponential jitter retry backoff returned by `exp_jitter(...)`.
class ExpJitterBackoffSpec(TypedDict):
    kind: Literal["exp_jitter"]
    min_s: float
    max_s: float
    jitter: str


# Retry payload returned by `retry(...)`.
class RetrySpec(TypedDict):
    attempts: int
    on_exit: list[int]
    backoff: FixedBackoffSpec | ExpJitterBackoffSpec


# Task-side facts exposed to custom placement policies.
class PolicyTaskContextSpec(TypedDict):
    side_effecting: bool


# Local-machine facts exposed to custom placement policies.
class PolicyLocalContextSpec(TypedDict):
    cpu_percent: float


# Policy input payload returned by `PolicyContext(...)`.
class PolicyContextSpec(TypedDict):
    task: PolicyTaskContextSpec
    local: PolicyLocalContextSpec


# Explicit local placement decision returned by `Decision.local(...)`.
class LocalDecisionSpec(TypedDict, total=False):
    mode: Literal["local"]
    reason: str
    local: LocalSpec


# Explicit remote placement decision returned by `Decision.remote(...)`.
class RemoteDecisionSpec(TypedDict):
    mode: Literal["remote"]
    reason: str
    remote: RemoteSpec


# Execution selector returned by `LocalOnly(...)`.
class LocalOnlyExecutionSpec(TypedDict):
    kind: Literal["local_only"]
    local: LocalSpec


# Execution selector returned by `RemoteOnly(...)`.
class RemoteOnlyExecutionSpec(TypedDict):
    kind: Literal["remote_only"]
    remote: RemoteSpec


# Execution selector returned by `ByCustomPolicy(...)`.
class ByCustomPolicyExecutionSpec(TypedDict, total=False):
    kind: Literal["by_custom_policy"]
    policy_name: str
    decision: LocalDecisionSpec | RemoteDecisionSpec


# Execution selector returned by `UseSession(...)`.
class UseSessionExecutionSpec(TypedDict):
    kind: Literal["use_session"]
    name: str
    cascade: bool


# Reuse the same workspace filesystem across tasks in one session.
class ShareWorkspaceReuseSpec(TypedDict):
    kind: Literal["share_workspace"]


# Persist only selected paths or globs across tasks in one session.
class SharePathsReuseSpec(TypedDict):
    kind: Literal["share_paths"]
    paths: list[PathSelector | GlobOutput]


# Named session returned by `session(...)`.
class SessionSpec(TypedDict):
    name: str
    execution: LocalOnlyExecutionSpec | RemoteOnlyExecutionSpec
    reuse: ShareWorkspaceReuseSpec | SharePathsReuseSpec
    lifetime: Literal["per_run"]
    context: CurrentStateSpec | None


# Command step returned by `cmd(...)`.
class CommandStepSpec(TypedDict):
    kind: Literal["cmd"]
    argv: list[str]
    cwd: str | None
    env: dict[str, str]


# Script step returned by `script(...)`.
class ScriptStepSpec(TypedDict):
    kind: Literal["script"]
    path: str
    argv: list[str]
    interpreter: str | None
    cwd: str | None
    env: dict[str, str]


# CurrentState input snapshot passed to task context hashing and remote execution.
class CurrentStateSpec(TypedDict):
    roots: list[PathSelector]
    ignored: list[PathSelector | GitignoreSource]
    include: list[PathSelector]


# Module-level task defaults merged during workspace loading.
class ModuleDefaults(TypedDict, total=False):
    queue: QueueUseSpec
    retry: RetrySpec
    container_runtime: ContainerRuntimeSpec | DockerfileRuntimeSpec
    tags: list[str]


# Task dictionary returned by `task(...)` after dependency normalization.
class TaskSpec(TypedDict):
    name: str
    deps: list[str]
    steps: list[CommandStepSpec | ScriptStepSpec]
    needs: list[NeedSpec]
    queue: QueueUseSpec | None
    retry: RetrySpec | None
    timeout_s: int | None
    context: CurrentStateSpec | None
    outputs: list[PathSelector | GlobOutput]
    execution: LocalOnlyExecutionSpec | RemoteOnlyExecutionSpec | ByCustomPolicyExecutionSpec | UseSessionExecutionSpec | None
    tags: list[str]
    doc: str


# Top-level TASKS.py module payload returned by `module_spec(...)`.
class ModuleSpec(TypedDict):
    spec_version: Literal[1]
    project_id: str | None
    tasks: list[TaskSpec]
    sessions: list[SessionSpec]
    limiters: list[ResourceLimiterSpec | LockLimiterSpec | RateLimitLimiterSpec | ProcessCapLimiterSpec]
    queues: list[QueueDefinition]
    exclude: list[str]
    includes: list[PathSelector]
    defaults: ModuleDefaults


# Machine-wide coordination scope.
MACHINE: Literal["machine"]
# User-wide coordination scope.
USER: Literal["user"]
# Project-wide coordination scope.
PROJECT: Literal["project"]
# Worktree-wide coordination scope.
WORKTREE: Literal["worktree"]
# Need hold mode that lasts for the whole task.
DURING: Literal["during"]
# Need hold mode that applies only at task start.
AT_START: Literal["at_start"]
# FIFO queue discipline.
FIFO: Literal["fifo"]
# Priority queue discipline.
PRIORITY: Literal["priority"]
# Workspace transfer mode for repository zip snapshots.
REPO_ZIP_SNAPSHOT: Literal["REPO_ZIP_SNAPSHOT"]
# Result sync mode that returns declared outputs plus logs.
OUTPUTS_AND_LOGS: Literal["OUTPUTS_AND_LOGS"]
# Per-run session lifetime. Cross-run sessions are not supported in v1.
PER_RUN: Literal["per_run"]


# Named placement reason constants used by custom placement policies.
class Reason:
    SIDE_EFFECTING_TASK: Literal["SIDE_EFFECTING_TASK"]
    NO_REMOTE_REACHABLE: Literal["NO_REMOTE_REACHABLE"]
    LOCAL_CPU_HIGH_ARM_IDLE: Literal["LOCAL_CPU_HIGH_ARM_IDLE"]
    LOCAL_CPU_HIGH: Literal["LOCAL_CPU_HIGH"]
    DEFAULT_LOCAL_POLICY: Literal["DEFAULT_LOCAL_POLICY"]


# Placement decision namespace used by custom placement policies.
# Only literal direct calls `Decision.local(...)` and `Decision.remote(...)`
# are supported by the loader. Do not alias `Decision` or its methods.
class Decision:
    # Return an explicit local placement decision from a custom policy.
    @staticmethod
    def local(
        local: LocalSpec | None = ...,
        reason: str = ...,
    ) -> LocalDecisionSpec: ...

    # Return an explicit remote placement decision from a custom policy.
    @staticmethod
    def remote(
        remote: RemoteSpec,
        reason: str = ...,
    ) -> RemoteDecisionSpec: ...

def module_spec(
    tasks: list[TaskSpec],
    sessions: list[SessionSpec] | None = ...,
    limiters: (
        list[
            ResourceLimiterSpec
            | LockLimiterSpec
            | RateLimitLimiterSpec
            | ProcessCapLimiterSpec
        ]
        | None
    ) = ...,
    queues: list[QueueDefinition] | None = ...,
    exclude: list[str] | None = ...,
    includes: list[PathSelector] | None = ...,
    defaults: ModuleDefaults | None = ...,
    project_id: str | None = ...,
) -> ModuleSpec: ...
def task(
    name: str,
    deps: list[str | TaskSpec] | str | TaskSpec | None = ...,
    steps: list[CommandStepSpec | ScriptStepSpec] | None = ...,
    needs: list[NeedSpec] | None = ...,
    queue: QueueUseSpec | None = ...,
    retry: RetrySpec | None = ...,
    timeout_s: int | None = ...,
    context: CurrentStateSpec | None = ...,
    outputs: list[PathSelector | GlobOutput] | None = ...,
    execution: (
        LocalOnlyExecutionSpec
        | RemoteOnlyExecutionSpec
        | ByCustomPolicyExecutionSpec
        | UseSessionExecutionSpec
        | None
    ) = ...,
    tags: list[str] | None = ...,
    doc: str | None = ...,
) -> TaskSpec: ...
def cmd(
    *argv: str,
    cwd: str | None = ...,
    env: dict[str, str] | None = ...,
) -> CommandStepSpec: ...
def script(
    path: str,
    *argv: str,
    interpreter: str | None = ...,
    cwd: str | None = ...,
    env: dict[str, str] | None = ...,
) -> ScriptStepSpec: ...
def need(
    name: str,
    slots: float = ...,
    scope: Literal["machine", "user", "project", "worktree"] = ...,
    hold: Literal["during", "at_start"] = ...,
) -> NeedSpec: ...
def queue_use(
    name: str,
    scope: Literal["machine", "user", "project", "worktree"] = ...,
    slots: int = ...,
    priority: int = ...,
) -> QueueUseSpec: ...
def resource(
    name: str,
    capacity: float,
    unit: str | None = ...,
    scope: Literal["machine", "user", "project", "worktree"] = ...,
) -> ResourceLimiterSpec: ...
def lock(
    name: str,
    scope: Literal["machine", "user", "project", "worktree"] = ...,
) -> LockLimiterSpec: ...
def queue_def(
    name: str,
    slots: int,
    discipline: Literal["fifo", "priority"] = ...,
    max_pending: int | None = ...,
    scope: Literal["machine", "user", "project", "worktree"] = ...,
) -> QueueDefinition: ...
def rate_limit(
    name: str,
    burst: int,
    refill_per_second: float,
    scope: Literal["machine", "user", "project", "worktree"] = ...,
) -> RateLimitLimiterSpec: ...
def process_cap(
    name: str,
    max_running: int,
    match: str | None = ...,
    scope: Literal["machine", "user", "project", "worktree"] = ...,
) -> ProcessCapLimiterSpec: ...
def retry(
    attempts: int = ...,
    on_exit: list[int] | None = ...,
    backoff: FixedBackoffSpec | ExpJitterBackoffSpec | None = ...,
) -> RetrySpec: ...
def fixed(seconds: float) -> FixedBackoffSpec: ...
def exp_jitter(
    min_s: float = ...,
    max_s: float = ...,
    jitter: str = ...,
) -> ExpJitterBackoffSpec: ...
def Local(
    id: str,
    max_parallel_tasks: int = ...,
    runtime: ContainerRuntimeSpec | DockerfileRuntimeSpec | None = ...,
) -> LocalSpec: ...
def Remote(
    pool: str | None = ...,
    required_tags: list[str] | None = ...,
    required_capabilities: list[str] | None = ...,
    transport: (
        DirectHttpsTransportSpec | AnyTransportSpec | TorOnionServiceTransportSpec | None
    ) = ...,
    runtime: ContainerRuntimeSpec | DockerfileRuntimeSpec | None = ...,
) -> RemoteSpec: ...
def DirectHttps() -> DirectHttpsTransportSpec: ...
def AnyTransport() -> AnyTransportSpec: ...
def TorOnionService() -> TorOnionServiceTransportSpec: ...
def ContainerRuntime(
    image: str,
    command: list[str] | None = ...,
    mounts: list[ContainerMountSpec] | None = ...,
    env: dict[str, str] | None = ...,
    resources: ContainerResourceLimitsSpec | None = ...,
) -> ContainerRuntimeSpec: ...
def DockerfileRuntime(
    dockerfile: PathSelector | str,
    build_context: PathSelector | str | None = ...,
    command: list[str] | None = ...,
    mounts: list[ContainerMountSpec] | None = ...,
    env: dict[str, str] | None = ...,
    resources: ContainerResourceLimitsSpec | None = ...,
) -> DockerfileRuntimeSpec: ...
def PolicyContext(
    task_side_effecting: bool = ...,
    local_cpu_percent: float = ...,
) -> PolicyContextSpec: ...
def LocalOnly(local: LocalSpec) -> LocalOnlyExecutionSpec: ...
def RemoteOnly(remote: RemoteSpec) -> RemoteOnlyExecutionSpec: ...
def ByCustomPolicy(policy: object) -> ByCustomPolicyExecutionSpec: ...
def ShareWorkspace() -> ShareWorkspaceReuseSpec: ...
def SharePaths(paths: list[PathSelector | GlobOutput]) -> SharePathsReuseSpec: ...
def session(
    name: str,
    execution: LocalOnlyExecutionSpec | RemoteOnlyExecutionSpec,
    reuse: ShareWorkspaceReuseSpec | SharePathsReuseSpec,
    lifetime: Literal["per_run"] = ...,
    context: CurrentStateSpec | None = ...,
) -> SessionSpec: ...
def UseSession(name: str, cascade: bool = ...) -> UseSessionExecutionSpec: ...
def path(value: str) -> PathSelector: ...
def glob(value: str) -> GlobOutput: ...
def gitignore() -> GitignoreSource: ...
def CurrentState(
    roots: list[PathSelector] | None = ...,
    ignored: list[PathSelector | GitignoreSource] | None = ...,
    include: list[PathSelector] | None = ...,
) -> CurrentStateSpec: ...
