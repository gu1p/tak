# Tak Agent Docs

## What Tak Is For

Tak is a task orchestrator for project-local `TASKS.py` workspaces. It loads the current directory's `TASKS.py`, follows explicit `module_spec(includes=[...])` links, builds one validated dependency graph, and executes local and remote work with consistent retry, timeout, and resource-coordination behavior.

Use this bundle when an agent needs to draft or review `TASKS.py` for another project. Start from the nearest example, then adapt the smallest pattern that matches the project shape.

## Core Capabilities

- Current-directory workspace loading with explicit `module_spec(includes=[...])` composition.
- Strict label parsing for absolute and relative task references.
- DAG validation (missing dependency and cycle detection) before execution.
- Command and script step execution with explicit `cwd` and `env` control.
- Retry policies with fixed or exponential-jitter backoff.
- Timeout controls per task.
- Client-side lease coordination for `needs` (resource/lock/rate/process/queue semantics).
- Remote execution with direct or Tor transport plus artifact roundtrip.
- Containerized runtimes from either a prebuilt image or a workspace `Dockerfile`.
- Hybrid local+remote pipelines with stable run summaries.

## TASKS.py API Surface

The shipped Python DSL surface is:

```pyi
MACHINE: str
USER: str
PROJECT: str
WORKTREE: str
DURING: str
AT_START: str
FIFO: str
PRIORITY: str

def module_spec(tasks: list, limiters: list | None = ..., queues: list | None = ..., exclude: list | None = ..., includes: list | None = ..., defaults: dict | None = ..., project_id: str | None = ...) -> dict: ...
def task(name: str, deps: list | str | dict | None = ..., steps: list | None = ..., needs: list | None = ..., queue: dict | None = ..., retry: dict | None = ..., timeout_s: int | None = ..., context: dict | None = ..., outputs: list | None = ..., execution: dict | None = ..., tags: list | None = ..., doc: str | None = ...) -> dict: ...
def cmd(*argv: str, cwd: str | None = ..., env: dict | None = ...) -> dict: ...
def script(path: str, *argv: str, interpreter: str | None = ..., cwd: str | None = ..., env: dict | None = ...) -> dict: ...
def need(name: str, slots: float = ..., scope: str = ..., hold: str = ...) -> dict: ...
def queue_use(name: str, scope: str = ..., slots: int = ..., priority: int = ...) -> dict: ...
def resource(name: str, capacity: float, unit: str | None = ..., scope: str = ...) -> dict: ...
def lock(name: str, scope: str = ...) -> dict: ...
def queue_def(name: str, slots: int, discipline: str = ..., max_pending: int | None = ..., scope: str = ...) -> dict: ...
def rate_limit(name: str, burst: int, refill_per_second: float, scope: str = ...) -> dict: ...
def process_cap(name: str, max_running: int, match: str | None = ..., scope: str = ...) -> dict: ...
def retry(attempts: int = ..., on_exit: list | None = ..., backoff: dict | None = ...) -> dict: ...
def fixed(seconds: float) -> dict: ...
def exp_jitter(min_s: float = ..., max_s: float = ..., jitter: str = ...) -> dict: ...
def Local(id: str, max_parallel_tasks: int = ..., runtime: dict | None = ...) -> dict: ...
def Remote(pool: str | None = ..., required_tags: list | None = ..., required_capabilities: list | None = ..., transport: dict | None = ..., runtime: dict | None = ...) -> dict: ...
def AnyTransport() -> dict: ...
def ContainerRuntime(image: str, command: list | None = ..., mounts: list | None = ..., env: dict | None = ..., resources: dict | None = ...) -> dict: ...
def DockerfileRuntime(dockerfile: dict | str, build_context: dict | str | None = ..., command: list | None = ..., mounts: list | None = ..., env: dict | None = ..., resources: dict | None = ...) -> dict: ...
def LocalOnly(local: dict) -> dict: ...
def RemoteOnly(remote: dict) -> dict: ...
def ByCustomPolicy(policy: object) -> dict: ...
def path(value: str) -> dict: ...
def glob(value: str) -> dict: ...
def gitignore() -> dict: ...
def CurrentState(roots: list | None = ..., ignored: list | None = ..., include: list | None = ...) -> dict: ...
```

## Project Patterns

- `artifact-pipeline`: start from `large/25_remote_direct_build_and_artifact_roundtrip`
- `ci-retry`: start from `small/08_retry_fixed_fail_once`
- `complex-pipeline`: start from `large/24_full_feature_matrix_end_to_end`
- `desktop-ui`: start from `medium/11_machine_lock_shared_ui`
- `hybrid-test-suite`: start from `large/28_hybrid_local_remote_test_suite_failure_with_logs`
- `monorepo`: start from `medium/18_multi_package_monorepo`, `large/24_full_feature_matrix_end_to_end`
- `multi-package`: start from `medium/18_multi_package_monorepo`
- `remote-build`: start from `large/25_remote_direct_build_and_artifact_roundtrip`
- `remote-ci`: start from `large/28_hybrid_local_remote_test_suite_failure_with_logs`
- `shared-machine`: start from `medium/11_machine_lock_shared_ui`
- `shell-heavy`: start from `small/04_cmd_with_env_and_cwd`
- `single-package`: start from `small/01_hello_single_task`, `small/04_cmd_with_env_and_cwd`, `small/08_retry_fixed_fail_once`
- `starter`: start from `small/01_hello_single_task`

## Example Chooser

### `small/01_hello_single_task`

- Use when: You need the smallest possible starting point for a new TASKS.py.
- Project shapes: starter, single-package
- Capabilities: basic-task, single-step, output-artifact
- Run target: `//:hello`
- Avoid when: You need retries, coordination, or multi-package composition.

### `small/04_cmd_with_env_and_cwd`

- Use when: Your task steps need explicit working-directory or environment control.
- Project shapes: single-package, shell-heavy
- Capabilities: cwd, env, local-command-steps
- Run target: `//:env_cmd`
- Avoid when: Retry policy or remote placement is the main concern.

### `small/08_retry_fixed_fail_once`

- Use when: A task is flaky and should retry only for selected exit codes.
- Project shapes: single-package, ci-retry
- Capabilities: retry, fixed-backoff, exit-code-matching
- Run target: `//:flaky_fixed`
- Avoid when: Failures should stay terminal with no retry.

### `medium/11_machine_lock_shared_ui`

- Use when: Multiple tasks share an exclusive local resource such as UI automation or a browser session.
- Project shapes: desktop-ui, shared-machine
- Capabilities: machine-lock, needs-coordination, shared-ui-resource
- Run target: `//:ui_test`
- Avoid when: No daemon-backed coordination is available.

### `medium/18_multi_package_monorepo`

- Use when: One root TASKS.py needs to compose tasks from apps or libs with explicit includes.
- Project shapes: monorepo, multi-package
- Capabilities: includes, cross-package-deps, monorepo
- Run target: `//apps/web:all`
- Avoid when: Everything fits cleanly in one package.

### `large/24_full_feature_matrix_end_to_end`

- Use when: You need a realistic end-to-end pipeline with multiple Tak features working together.
- Project shapes: complex-pipeline, monorepo
- Capabilities: defaults, queues, scripts, full-pipeline
- Run target: `//apps/qa:release`
- Avoid when: You only need a minimal starter or one isolated feature.

### `large/25_remote_direct_build_and_artifact_roundtrip`

- Use when: Build should run remotely and return declared outputs for local follow-up or verification.
- Project shapes: remote-build, artifact-pipeline
- Capabilities: remote-only, artifact-roundtrip, direct-remote, local-verify
- Run target: `//services/api:release`
- Avoid when: All work should stay local or remote infrastructure is unavailable.

### `large/28_hybrid_local_remote_test_suite_failure_with_logs`

- Use when: A pipeline mixes local prep with remote test execution and you need strong failure artifacts.
- Project shapes: hybrid-test-suite, remote-ci
- Capabilities: hybrid-local-remote, failure-diagnostics, log-retention
- Run target: `//apps/web:remote_suite`
- Avoid when: You need only a success-path remote example.

## Authoring Workflow

1. Pick the closest example from the chooser.
2. Start from a small `module_spec(...)` and add only the execution, retry, context, and coordination features the project actually needs.
3. Validate the graph and labels before running anything expensive.
4. Keep task docs, outputs, and dependencies explicit.

Starter shape from the Tak README:

```python
build = task(
    "build",
    steps=[cmd("sh", "-c", "mkdir -p out && echo build > out/build.log")],
)

test = task(
    "test",
    deps=[":build"],
    retry=retry(attempts=2, on_exit=[42], backoff=fixed(0.2)),
    timeout_s=120,
    steps=[cmd("sh", "-c", "echo test > out/test.log")],
)

SPEC = module_spec(
    project_id="hello_project",
    tasks=[build, test],
    limiters=[lock("ci_lock", scope=MACHINE)],
)
SPEC
```

Standard inspection workflow:

```bash
tak list
tak explain <target>
tak graph <target> --format dot
tak web <target>
tak run <target>
```

Smallest explicit root-task flow:

```bash
tak list
tak explain //:hello
tak graph //:hello --format dot
tak run //:hello
```
