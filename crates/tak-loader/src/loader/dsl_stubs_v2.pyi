from typing import Literal
from typing_extensions import TypedDict


class PathSelector(TypedDict):
    kind: Literal["path"]
    value: str


class GlobOutput(TypedDict):
    kind: Literal["glob"]
    value: str


class CommandStepSpec(TypedDict):
    kind: Literal["cmd"]
    argv: list[str]
    cwd: str | None
    env: dict[str, str]


class ScriptStepSpec(TypedDict):
    kind: Literal["script"]
    path: str
    argv: list[str]
    interpreter: str | None
    cwd: str | None
    env: dict[str, str]


class RemoteSelectionSpec(TypedDict):
    kind: Literal["balanced", "sequential", "round_robin"]


class TransportSpec(TypedDict):
    kind: Literal["direct", "any", "tor"]


class AffinitySpec(TypedDict):
    kind: Literal["prefer_same_node", "require_same_node"]
    group: str


class ScopedReference(TypedDict):
    name: str
    scope: Literal["machine", "user", "project", "worktree"]


class NeedSpec(TypedDict):
    limiter: ScopedReference
    slots: float
    hold: Literal["during", "at_start"]


class QueueUseSpec(TypedDict):
    queue: ScopedReference
    slots: int
    priority: int


class ResourceLimiterSpec(TypedDict):
    kind: Literal["resource"]
    name: str
    scope: Literal["machine", "user", "project", "worktree"]
    capacity: float
    unit: str | None


class LockLimiterSpec(TypedDict):
    kind: Literal["lock"]
    name: str
    scope: Literal["machine", "user", "project", "worktree"]


class RateLimitSpec(TypedDict):
    kind: Literal["rate_limit"]
    name: str
    scope: Literal["machine", "user", "project", "worktree"]
    burst: int
    refill_per_second: float


class ProcessCapSpec(TypedDict):
    kind: Literal["process_cap"]
    name: str
    scope: Literal["machine", "user", "project", "worktree"]
    max_running: int
    match: str | None


class QueueDefinition(TypedDict):
    name: str
    scope: Literal["machine", "user", "project", "worktree"]
    slots: int
    discipline: Literal["fifo", "priority"]
    max_pending: int | None


class FixedBackoffSpec(TypedDict):
    kind: Literal["fixed"]
    seconds: float


class ExpJitterBackoffSpec(TypedDict):
    kind: Literal["exp_jitter"]
    min_s: float
    max_s: float
    jitter: str


class RetrySpec(TypedDict):
    attempts: int
    on_exit: list[int]
    backoff: FixedBackoffSpec | ExpJitterBackoffSpec


class ReuseSpec(TypedDict, total=False):
    kind: Literal["workspace", "paths", "shared_workspace", "container"]
    paths: list[PathSelector | GlobOutput]
    max_parallel_tasks: int


class SessionSpec(TypedDict):
    __tak_kind: Literal["session_v2"]
    id: str
    name: str | None
    execution: "ExecutionSpec | None"
    reuse: ReuseSpec
    context: object | None
    affinity: AffinitySpec | None


class LocalSpec(TypedDict):
    container: object | None
    session: SessionSpec | None


class RemoteSpec(TypedDict):
    pool: str | None
    required_tags: list[str]
    required_capabilities: list[str]
    transport: TransportSpec | None
    container: object | None
    selection: RemoteSelectionSpec
    session: SessionSpec | None


class ExecutionSpec(TypedDict, total=False):
    kind: Literal["local_only", "remote_only"]
    local: LocalSpec
    remote: RemoteSpec


class DefaultsSpec(TypedDict):
    __tak_kind: Literal["defaults_v2"]
    queue: QueueUseSpec | None
    retry: RetrySpec | None
    container: object | None
    execution: ExecutionSpec | None
    tags: list[str]
    pass_env: list[str]


class TaskSpec(TypedDict):
    name: str
    deps: list[str]
    steps: list[CommandStepSpec | ScriptStepSpec]
    needs: list[NeedSpec]
    queue: QueueUseSpec | None
    retry: RetrySpec | None
    timeout_s: int | None
    context: object | None
    outputs: list[PathSelector | GlobOutput]
    execution: ExecutionSpec | None
    session: SessionSpec | None
    cascade_session: bool
    tags: list[str]
    doc: str
    idempotent: bool
    pass_env: list[str]
    affinity: AffinitySpec | None


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


class RemoteSelection:
    @staticmethod
    def Balanced() -> RemoteSelectionSpec: ...
    @staticmethod
    def Sequential() -> RemoteSelectionSpec: ...
    @staticmethod
    def RoundRobin() -> RemoteSelectionSpec: ...


class Scope:
    Machine: Literal["machine"]
    User: Literal["user"]
    Project: Literal["project"]
    Worktree: Literal["worktree"]


class Hold:
    During: Literal["during"]
    AtStart: Literal["at_start"]


class QueueDiscipline:
    Fifo: Literal["fifo"]
    Priority: Literal["priority"]


class Affinity:
    @staticmethod
    def PreferSameNode(group: str) -> AffinitySpec: ...
    @staticmethod
    def RequireSameNode(group: str) -> AffinitySpec: ...


class Transport:
    @staticmethod
    def DirectHttps() -> TransportSpec: ...
    @staticmethod
    def Any() -> TransportSpec: ...
    @staticmethod
    def TorOnionService() -> TransportSpec: ...


class SessionReuse:
    @staticmethod
    def Workspace() -> ReuseSpec: ...
    @staticmethod
    def Paths(paths: list[PathSelector | GlobOutput]) -> ReuseSpec: ...
    @staticmethod
    def SharedWorkspace(max_parallel_tasks: int) -> ReuseSpec: ...
    @staticmethod
    def Container() -> ReuseSpec: ...


class Execution:
    @staticmethod
    def Local(container: object | None = ..., session: SessionSpec | None = ...) -> ExecutionSpec: ...
    @staticmethod
    def Remote(pool: str | None = ..., required_tags: list[str] | None = ..., required_capabilities: list[str] | None = ..., transport: TransportSpec | None = ..., container: object | None = ..., selection: RemoteSelectionSpec | None = ..., session: SessionSpec | None = ...) -> ExecutionSpec: ...


def Defaults(container: object | None = ..., execution: ExecutionSpec | None = ..., retry: RetrySpec | None = ..., queue: QueueUseSpec | None = ..., tags: list[str] | None = ..., pass_env: list[str] | None = ...) -> DefaultsSpec: ...
def module_spec(tasks: list[TaskSpec], *, spec_version: Literal[2], limiters: list[ResourceLimiterSpec | LockLimiterSpec | RateLimitSpec | ProcessCapSpec] | None = ..., queues: list[QueueDefinition] | None = ..., exclude: list[str] | None = ..., includes: list[PathSelector] | None = ..., defaults: DefaultsSpec | None = ..., project_id: str | None = ...) -> ModuleSpec: ...
def session(name: str | None = ..., execution: ExecutionSpec | None = ..., reuse: ReuseSpec | None = ..., context: object | None = ..., affinity: AffinitySpec | None = ...) -> SessionSpec: ...
def task(name: str, deps: list[str | TaskSpec] | str | TaskSpec | None = ..., steps: list[CommandStepSpec | ScriptStepSpec] | None = ..., needs: list[NeedSpec] | None = ..., queue: QueueUseSpec | None = ..., retry: RetrySpec | None = ..., timeout_s: int | None = ..., context: object | None = ..., outputs: list[PathSelector | GlobOutput] | None = ..., execution: ExecutionSpec | None = ..., use_session: SessionSpec | None = ..., cascade_session: bool = ..., tags: list[str] | None = ..., doc: str | None = ..., idempotent: bool = ..., pass_env: list[str] | None = ..., affinity: AffinitySpec | None = ...) -> TaskSpec: ...
def cmd(*argv: str, cwd: str | None = ..., env: dict[str, str] | None = ...) -> CommandStepSpec: ...
def script(path: str, *argv: str, interpreter: str | None = ..., cwd: str | None = ..., env: dict[str, str] | None = ...) -> ScriptStepSpec: ...
def path(value: str) -> PathSelector: ...
def glob(value: str) -> GlobOutput: ...
def need(name: str, slots: float = ..., scope: Literal["machine", "user", "project", "worktree"] = ..., hold: Literal["during", "at_start"] = ...) -> NeedSpec: ...
def queue_use(name: str, scope: Literal["machine", "user", "project", "worktree"] = ..., slots: int = ..., priority: int = ...) -> QueueUseSpec: ...
def resource(name: str, capacity: float, unit: str | None = ..., scope: Literal["machine", "user", "project", "worktree"] = ...) -> ResourceLimiterSpec: ...
def lock(name: str, scope: Literal["machine", "user", "project", "worktree"] = ...) -> LockLimiterSpec: ...
def queue_def(name: str, slots: int, discipline: Literal["fifo", "priority"] = ..., max_pending: int | None = ..., scope: Literal["machine", "user", "project", "worktree"] = ...) -> QueueDefinition: ...
def rate_limit(name: str, burst: int, refill_per_second: float, scope: Literal["machine", "user", "project", "worktree"] = ...) -> RateLimitSpec: ...
def process_cap(name: str, max_running: int, match: str | None = ..., scope: Literal["machine", "user", "project", "worktree"] = ...) -> ProcessCapSpec: ...
def retry(attempts: int = ..., on_exit: list[int] | None = ..., backoff: FixedBackoffSpec | ExpJitterBackoffSpec | None = ...) -> RetrySpec: ...
def fixed(seconds: float) -> FixedBackoffSpec: ...
def exp_jitter(min_s: float = ..., max_s: float = ..., jitter: str = ...) -> ExpJitterBackoffSpec: ...
