from typing import Literal
from typing_extensions import TypedDict


# One explicit workspace path selected with `path(...)`.
class PathSelector(TypedDict):
    kind: Literal["path"]
    value: str


# One explicit workspace glob selected with `glob(...)`.
class GlobOutput(TypedDict):
    kind: Literal["glob"]
    value: str


# One bind mount entry for a container job.
class ContainerMountSpec(TypedDict):
    source: str
    target: str
    read_only: bool


# CPU and memory reservations for a container job.
class ContainerResourceLimitsSpec:
    cpu_cores: float
    memory_mb: int


# Container job built from a prebuilt image.
class ImageContainerSpec(TypedDict):
    kind: Literal["containerized"]
    image: str
    dockerfile: None
    build_context: None
    mounts: list[ContainerMountSpec]
    env: dict[str, str]
    resource_limits: ContainerResourceLimitsSpec | None


# Container job built from a workspace Dockerfile.
class DockerfileContainerSpec(TypedDict):
    kind: Literal["containerized"]
    image: None
    dockerfile: PathSelector
    build_context: PathSelector
    mounts: list[ContainerMountSpec]
    env: dict[str, str]
    resource_limits: ContainerResourceLimitsSpec | None


# Command step returned by `cmd(...)`.
class CommandStepSpec(TypedDict):
    kind: Literal["cmd"]
    argv: list[str]
    cwd: str | None
    env: dict[str, str]


# Workspace script step returned by `script(...)`.
class ScriptStepSpec(TypedDict):
    kind: Literal["script"]
    path: str
    argv: list[str]
    interpreter: str | None
    cwd: str | None
    env: dict[str, str]


# Remote worker ordering selected by `RemoteSelection`.
class RemoteSelectionSpec(TypedDict):
    kind: Literal["balanced", "sequential", "round_robin"]


# Direct, Tor, or any-transport remote worker filter.
class TransportSpec(TypedDict):
    kind: Literal["direct", "any", "tor"]


# Soft or hard same-node placement constraint.
class AffinitySpec(TypedDict):
    kind: Literal["prefer_same_node", "require_same_node"]
    group: str


# Name and scope shared by limiter and queue references.
class ScopedReference(TypedDict):
    name: str
    scope: Literal["machine", "user", "project", "worktree"]


# Limiter lease request returned by `need(...)`.
class NeedSpec(TypedDict):
    limiter: ScopedReference
    slots: float
    hold: Literal["during", "at_start"]


# Queue admission request returned by `queue_use(...)`.
class QueueUseSpec(TypedDict):
    queue: ScopedReference
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


# Token-bucket task-start limiter returned by `rate_limit(...)`.
class RateLimitSpec(TypedDict):
    kind: Literal["rate_limit"]
    name: str
    scope: Literal["machine", "user", "project", "worktree"]
    burst: int
    refill_per_second: float


# External-process limiter returned by `process_cap(...)`.
class ProcessCapSpec(TypedDict):
    kind: Literal["process_cap"]
    name: str
    scope: Literal["machine", "user", "project", "worktree"]
    max_running: int
    match: str | None


# Daemon queue definition returned by `queue_def(...)`.
class QueueDefinition(TypedDict):
    name: str
    scope: Literal["machine", "user", "project", "worktree"]
    slots: int
    discipline: Literal["fifo", "priority"]


# Fixed retry delay returned by `fixed(...)`.
class FixedBackoffSpec(TypedDict):
    kind: Literal["fixed"]
    seconds: float


# Bounded exponential retry delay returned by `exp_jitter(...)`.
class ExpJitterBackoffSpec(TypedDict):
    kind: Literal["exp_jitter"]
    min_s: float
    max_s: float
    jitter: str


# Retry contract returned by `retry(...)`.
class RetrySpec(TypedDict):
    attempts: int
    on_exit: list[int]
    backoff: FixedBackoffSpec | ExpJitterBackoffSpec


# Workspace, cache-path, shared-workspace, or fused-container session reuse.
class ReuseSpec(TypedDict, total=False):
    kind: Literal["workspace", "paths", "shared_workspace", "container"]
    paths: list[PathSelector | GlobOutput]
    max_parallel_tasks: int


# Repository gitignore rules used by `CurrentState(...)`.
class GitignoreSource(TypedDict):
    kind: Literal["gitignore"]


# Current workspace input snapshot sent with a daemon-owned run.
class CurrentStateSpec(TypedDict):
    roots: list[PathSelector]
    ignored: list[PathSelector | GitignoreSource]
    include: list[PathSelector]


# Per-run session declaration returned by `session(...)`.
class SessionSpec(TypedDict):
    __tak_kind: Literal["session_v2"]
    id: str
    name: str | None
    execution: "ExecutionSpec | None"
    reuse: ReuseSpec
    context: CurrentStateSpec | None
    affinity: AffinitySpec | None


# Concrete local placement submitted to takd.
class LocalSpec(TypedDict):
    reason: str
    container: object | None
    session: SessionSpec | None


# Concrete remote placement filters submitted to takd.
class RemoteSpec(TypedDict):
    reason: str
    pool: str | None
    required_tags: list[str]
    required_capabilities: list[str]
    transport: TransportSpec | None
    container: object | None
    selection: RemoteSelectionSpec
    session: SessionSpec | None


# Concrete placement or ordered placement candidates resolved by Tak.
class ExecutionSpec(TypedDict, total=False):
    kind: Literal["local_only", "remote_only", "first_available"]
    local: LocalSpec
    remote: RemoteSpec
    policy_id: str
    placements: list["ExecutionSpec"]


# Task facts exposed to a Python placement policy.
class TaskPolicyContext(TypedDict):
    side_effecting: bool


# Local-machine facts exposed to a Python placement policy.
class LocalPolicyContext(TypedDict):
    cpu_percent: float


# Policy input returned by `PolicyContext(...)`.
class PolicyContextSpec(TypedDict):
    task: TaskPolicyContext
    local: LocalPolicyContext


# Version 2 module defaults returned by `Defaults(...)`.
class DefaultsSpec(TypedDict):
    __tak_kind: Literal["defaults_v2"]
    queue: QueueUseSpec | None
    retry: RetrySpec | None
    container: object | None
    execution: ExecutionSpec | None
    tags: list[str]
    pass_env: list[str]


# Version 2 task dictionary returned by `task(...)`.
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
    execution: ExecutionSpec | None
    session: SessionSpec | None
    cascade_session: bool
    tags: list[str]
    doc: str
    idempotent: bool
    pass_env: list[str]
    affinity: AffinitySpec | None


# Top-level version 2 TASKS.py payload returned by `module_spec(...)`.
class ModuleSpec(TypedDict):
    __tak_kind: Literal["module_spec_v2"]
    spec_version: Literal[2]
    project_id: str | None
    tasks: list[TaskSpec]
    limiters: list[ResourceLimiterSpec | LockLimiterSpec | RateLimitSpec | ProcessCapSpec]
    queues: list[QueueDefinition]
    exclude: list[str]
    includes: list[PathSelector]
    defaults: DefaultsSpec


# Remote worker selection strategies.
class RemoteSelection:
    # Balance work across least-loaded matching workers. This is the default.
    @staticmethod
    def Balanced() -> RemoteSelectionSpec: ...
    # Try matching workers in stable inventory order.
    @staticmethod
    def Sequential() -> RemoteSelectionSpec: ...
    # Rotate through matching workers with daemon-persisted state.
    @staticmethod
    def RoundRobin() -> RemoteSelectionSpec: ...


# Coordination scope constants.
class Scope:
    # Machine-wide coordination scope.
    Machine: Literal["machine"]
    # User-wide coordination scope.
    User: Literal["user"]
    # Project-wide coordination scope.
    Project: Literal["project"]
    # Worktree-wide coordination scope.
    Worktree: Literal["worktree"]


# Limiter hold-mode constants.
class Hold:
    # Hold limiter capacity throughout the task.
    During: Literal["during"]
    # Consume limiter capacity only when starting the task.
    AtStart: Literal["at_start"]


# Queue scheduling discipline constants.
class QueueDiscipline:
    # Admit waiting tasks in first-in, first-out order.
    Fifo: Literal["fifo"]
    # Admit higher-priority waiting tasks first.
    Priority: Literal["priority"]


# Same-worker placement constraints.
class Affinity:
    # Prefer placing affinity-group tasks on one worker.
    @staticmethod
    def PreferSameNode(group: str) -> AffinitySpec: ...
    # Require every affinity-group task to use one worker.
    @staticmethod
    def RequireSameNode(group: str) -> AffinitySpec: ...


# Remote worker transport filters.
class Transport:
    # Require direct HTTPS transport.
    @staticmethod
    def DirectHttps() -> TransportSpec: ...
    # Allow daemon inventory to select direct or Tor transport.
    @staticmethod
    def Any() -> TransportSpec: ...
    # Require Tor onion-service transport.
    @staticmethod
    def TorOnionService() -> TransportSpec: ...


# Per-run session filesystem and process reuse modes.
class SessionReuse:
    # Give every task an isolated workspace.
    @staticmethod
    def Workspace() -> ReuseSpec: ...
    # Reuse only private-CAS cache paths between tasks.
    @staticmethod
    def Paths(paths: list[PathSelector | GlobOutput]) -> ReuseSpec: ...
    # Share one session workspace with bounded task concurrency.
    @staticmethod
    def SharedWorkspace(max_parallel_tasks: int) -> ReuseSpec: ...
    # Fuse a cascaded task graph into one container job.
    @staticmethod
    def Container() -> ReuseSpec: ...


# Container runtime declarations.
class Container:
    # Declare CPU and memory reservations.
    @staticmethod
    def Resources(cpu_cores: float, memory_mb: int) -> ContainerResourceLimitsSpec: ...
    # Run one job inside a prebuilt image.
    @staticmethod
    def Image(image: str, mounts: list[ContainerMountSpec] | None = ..., env: dict[str, str] | None = ..., resources: ContainerResourceLimitsSpec | None = ...) -> ImageContainerSpec: ...
    # Build one job container from a workspace Dockerfile.
    @staticmethod
    def Dockerfile(dockerfile: PathSelector | str, build_context: PathSelector | str | None = ..., mounts: list[ContainerMountSpec] | None = ..., env: dict[str, str] | None = ..., resources: ContainerResourceLimitsSpec | None = ...) -> DockerfileContainerSpec: ...


# Placement constructors resolved by Tak before daemon submission.
class Execution:
    # Force daemon-owned scheduling onto the local worker.
    @staticmethod
    def Local(container: object | None = ..., session: SessionSpec | None = ...) -> ExecutionSpec: ...
    # Force daemon-owned scheduling onto matching remote workers.
    @staticmethod
    def Remote(pool: str | None = ..., required_tags: list[str] | None = ..., required_capabilities: list[str] | None = ..., transport: TransportSpec | None = ..., container: object | None = ..., selection: RemoteSelectionSpec | None = ..., session: SessionSpec | None = ...) -> ExecutionSpec: ...
    # Submit concrete placement candidates in authored preference order.
    @staticmethod
    def FirstAvailable(placements: list[ExecutionSpec], doc: str | None = ..., name: str | None = ...) -> ExecutionSpec: ...
    # Resolve one Python policy to a concrete placement before submission.
    @staticmethod
    def Decide(policy: object) -> ExecutionSpec: ...


# Placement results returned from custom Python policies.
class Decision:
    # Return an explicit local placement decision from a custom policy.
    @staticmethod
    def local(reason: str = ..., container: object | None = ...) -> ExecutionSpec: ...
    # Return an explicit remote placement decision from a custom policy.
    @staticmethod
    def remote(reason: str = ..., pool: str | None = ..., required_tags: list[str] | None = ..., required_capabilities: list[str] | None = ..., transport: TransportSpec | None = ..., container: object | None = ...) -> ExecutionSpec: ...


def Defaults(container: object | None = ..., execution: ExecutionSpec | None = ..., retry: RetrySpec | None = ..., queue: QueueUseSpec | None = ..., tags: list[str] | None = ..., pass_env: list[str] | None = ...) -> DefaultsSpec: ...
def PolicyContext(task_side_effecting: bool = ..., local_cpu_percent: float = ...) -> PolicyContextSpec: ...
def module_spec(tasks: list[TaskSpec], *, spec_version: Literal[2], limiters: list[ResourceLimiterSpec | LockLimiterSpec | RateLimitSpec | ProcessCapSpec] | None = ..., queues: list[QueueDefinition] | None = ..., exclude: list[str] | None = ..., includes: list[PathSelector] | None = ..., defaults: DefaultsSpec | None = ..., project_id: str | None = ...) -> ModuleSpec: ...
def session(name: str | None = ..., execution: ExecutionSpec | None = ..., reuse: ReuseSpec | None = ..., context: CurrentStateSpec | None = ..., affinity: AffinitySpec | None = ...) -> SessionSpec: ...
def task(name: str, deps: list[str | TaskSpec] | str | TaskSpec | None = ..., steps: list[CommandStepSpec | ScriptStepSpec] | None = ..., needs: list[NeedSpec] | None = ..., queue: QueueUseSpec | None = ..., retry: RetrySpec | None = ..., timeout_s: int | None = ..., context: CurrentStateSpec | None = ..., outputs: list[PathSelector | GlobOutput] | None = ..., execution: ExecutionSpec | None = ..., use_session: SessionSpec | None = ..., cascade_session: bool = ..., tags: list[str] | None = ..., doc: str | None = ..., idempotent: bool = ..., pass_env: list[str] | None = ..., affinity: AffinitySpec | None = ...) -> TaskSpec: ...
def cmd(*argv: str, cwd: str | None = ..., env: dict[str, str] | None = ...) -> CommandStepSpec: ...
def script(path: str, *argv: str, interpreter: str | None = ..., cwd: str | None = ..., env: dict[str, str] | None = ...) -> ScriptStepSpec: ...
def path(value: str) -> PathSelector: ...
def glob(value: str) -> GlobOutput: ...
def gitignore() -> GitignoreSource: ...
def CurrentState(roots: list[PathSelector] | None = ..., ignored: list[PathSelector | GitignoreSource] | None = ..., include: list[PathSelector] | None = ...) -> CurrentStateSpec: ...
def need(name: str, slots: float = ..., scope: Literal["machine", "user", "project", "worktree"] = ..., hold: Literal["during", "at_start"] = ...) -> NeedSpec: ...
def queue_use(name: str, scope: Literal["machine", "user", "project", "worktree"] = ..., slots: int = ..., priority: int = ...) -> QueueUseSpec: ...
def resource(name: str, capacity: float, unit: str | None = ..., scope: Literal["machine", "user", "project", "worktree"] = ...) -> ResourceLimiterSpec: ...
def lock(name: str, scope: Literal["machine", "user", "project", "worktree"] = ...) -> LockLimiterSpec: ...
def queue_def(name: str, slots: int, discipline: Literal["fifo", "priority"] = ..., scope: Literal["machine", "user", "project", "worktree"] = ...) -> QueueDefinition: ...
def rate_limit(name: str, burst: int, refill_per_second: float, scope: Literal["machine", "user", "project", "worktree"] = ...) -> RateLimitSpec: ...
def process_cap(name: str, max_running: int, match: str | None = ..., scope: Literal["machine", "user", "project", "worktree"] = ...) -> ProcessCapSpec: ...
def retry(attempts: int = ..., on_exit: list[int] | None = ..., backoff: FixedBackoffSpec | ExpJitterBackoffSpec | None = ...) -> RetrySpec: ...
def fixed(seconds: float) -> FixedBackoffSpec: ...
def exp_jitter(min_s: float = ..., max_s: float = ..., jitter: str = ...) -> ExpJitterBackoffSpec: ...
