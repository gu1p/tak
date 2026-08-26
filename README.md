# Tak

Tak is a task orchestrator for project-local `TASKS.py` workspaces and ordinary Makefiles. It can load a Tak-authored dependency graph, wrap one opaque `make <goal>` invocation, or promote explicitly annotated Make prerequisites into parallel Tak tasks.

## Why Teams Use Tak

- Keep task definitions close to code while still running one global graph.
- Coordinate shared machine resources (`cpu`, `ram`, locks, queues, rate limits, process caps) without custom glue scripts.
- Standardize execution behavior (timeouts, retries, remote placement, artifact sync) across local dev and CI.
- Keep failure diagnostics actionable with deterministic outputs and logs.

## Core Capabilities

- Current-directory workspace loading with explicit `module_spec(includes=[...])` composition.
- Makefile goal execution without requiring `TASKS.py`.
- Strict label parsing for absolute and relative task references.
- DAG validation (missing dependency and cycle detection) before execution.
- Command and script step execution with explicit `cwd` and `env` control.
- Retry policies with fixed or exponential-jitter backoff.
- Timeout controls per task.
- Client-side lease coordination for `needs` (resource/lock/rate/process/queue semantics).
- Remote containerized execution with direct or Tor transport plus artifact roundtrip.
- Containerized runtimes from either a prebuilt image or a workspace `Dockerfile`.
- Hybrid local+remote pipelines with stable run summaries.

## Hero Example Path

Start with these 8 examples in order:

1. [`small/01_hello_single_task`](examples/small/01_hello_single_task/README.md)
2. [`small/04_cmd_with_env_and_cwd`](examples/small/04_cmd_with_env_and_cwd/README.md)
3. [`small/08_retry_fixed_fail_once`](examples/small/08_retry_fixed_fail_once/README.md)
4. [`medium/11_machine_lock_shared_ui`](examples/medium/11_machine_lock_shared_ui/README.md)
5. [`medium/18_multi_package_monorepo`](examples/medium/18_multi_package_monorepo/README.md)
6. [`large/24_full_feature_matrix_end_to_end`](examples/large/24_full_feature_matrix_end_to_end/README.md)
7. [`large/25_remote_direct_build_and_artifact_roundtrip`](examples/large/25_remote_direct_build_and_artifact_roundtrip/README.md)
8. [`large/28_hybrid_local_remote_test_suite_failure_with_logs`](examples/large/28_hybrid_local_remote_test_suite_failure_with_logs/README.md)

For the full matrix (including reference scenarios), see [`examples/README.md`](examples/README.md) and [`examples/catalog.toml`](examples/catalog.toml).

## Phased Ergonomics Guide

- [`docs/ergonomics-and-distribution-phases.md`](docs/ergonomics-and-distribution-phases.md)
  - One document covering what Tak already ships today, what should come next, and the bigger distributed execution vision.

## CLI Quick Reference

- `tak list`
  - Enumerate workspace tasks with their fully-qualified labels, dependencies, and authored descriptions.
- `tak tree`
  - Render tasks as a tree for quick topology inspection.
- `tak docs dump`
  - Print the embedded agent-oriented Tak authoring bundle with DSL surface and example chooser.
- `tak explain <label>`
  - Show task composition (`deps`, `steps`, `needs`, timeout, retry).
- `tak graph [label] --format dot`
  - Print DOT graph for Graphviz or pipeline tooling.
- `tak web [label]`
  - Serve an interactive dependency graph UI locally. This is a graph viewer, not a remote-operations client.
- `tak make <goal>`
  - Run an ordinary Makefile goal through Tak; annotated phony prerequisites may fan out in parallel.
- `tak make --remote <goal>`
  - Force the whole `make <goal>` invocation onto a remote container; the Makefile may also declare this with `# tak:` comments.
- `tak run <label...>`
  - Execute targets and dependencies.
- `tak run hello`
  - At a workspace root, bare task names are shorthand for root-package labels such as `//:hello`.
- `tak run <label...> -j <N> --keep-going`
  - Configure parallelism and continue with independent work after failures.
- `tak run //:check`
  - Run the repo-owned quality gate declared in `TASKS.py`.
- `tak run --local-no-container <label...>`
  - Force local host execution and ignore declared container runtimes.
- `tak run --local --container <label...>`
  - Force local containerized execution using the declared or supplied container runtime.
- `tak run --remote <label...>`
  - Force remote containerized execution.
- `tak run .`
  - Invalid input. Use `tak list` first, then pass a real label such as `//:task` or `//pkg:task`.
- `--keep-going`
  - Continue independent tasks even after one target fails.
- `tak status`
  - Show local task/container/resource status plus configured remote node status.
- `tak update` / `tak update --check`
  - Update the installed `tak`/`takd` binaries from signed GitHub releases (or just report whether a newer version exists). `takd` agents can also auto-update themselves; see [Self-Update](docs/self-update.md).
- `tak local status`
  - Show local active tasks, containerized runs, CPU, RAM, storage, and optional local daemon lease status.
- `tak remote add <token>`
  - Import a secret `takd` agent invite/token into local client config.
- `tak remote add`
  - Open an interactive terminal flow for adding a remote from words, a token, or a secret Tor invite/address.
- `tak remote add --words <word>...`
  - Import a secret Tor v3 `takd` invite from the 19-word manual-entry phrase emitted by `takd token show --words`.
- `tak remote add --words`
  - Open the interactive word-entry flow directly.
- `tak remote scan`
  - Pick a camera, preview its feed in the terminal, and add a remote from a scanned QR token.
- `tak remote list`
  - Show configured remote agents in client priority order.
- `tak remote remove <node-id>`
  - Remove one configured remote agent from local client config.
- `tak remote status`
  - Show running jobs, containers, CPU, RAM, storage, and image-cache usage for configured remote agents.
- `tak remote status --watch --interval-ms <N>`
  - Refresh remote node status continuously in a dynamic terminal dashboard.
- `tak remote logs --node <id>`
  - Print the service log from one configured remote node.
- `tak remote tasks --node <id>`
  - List task attempts known by one configured remote node.
- `tak remote task logs --node <id> <task-run-id>`
  - Print persisted stdout/stderr for one task run on a configured remote node.
- `tak task list`
  - List task runs initiated by this local Tak client.
- `tak task logs <task-run-id>`
  - Print captured stdout/stderr for one locally initiated task run.
- `takd init`
  - Create Tor-first agent identity and hidden-service runtime state.
- `takd serve`
  - Start the standalone execution agent service and publish its hidden-service token when ready.
- `takd status`
  - Show the agent transport/readiness plus the resolved `service.log` path and whether it exists yet.
- `takd logs`
  - Print the most recent server-side `takd` log lines from the agent state directory.
- `takd tasks`
  - List tasks currently executing in the running local `takd` process.
- `takd token show`
  - Reprint the persisted secret onboarding invite/token, or wait until it is advertised with `--wait`.
- `takd token show --words`
  - Print the secret 19-word Tor v3 onboarding phrase for manual typing.
- `takd token show --words-table`
  - Print the same Tor v3 onboarding phrase as numbered cells for human copying.
- `takd token show --qr`
  - Render the onboarding token as a terminal QR code plus the exact `tak remote add '...'` command, and show numbered word cells when the invite targets a real Tor v3 onion host.

## Tor Remote Security Model

- The Tor invite/address is a secret, not just a location.
- Anyone with it can submit jobs and read outputs/logs.
- Do not paste it into shared chats, issue trackers, screenshots, or logs.
- Rotate the onion address if exposed.
- Tak remote does not provide multi-user isolation.

## Run Output Signals

`tak run` streams task `stdout` and `stderr` live as work executes, then prints one summary line per executed task. Remote and containerized runs use the same local-terminal contract so output stays visible while the task is still running.

Example:

```text
apps/web:test: ok (attempts=1, exit_code=0, placement=remote, remote_node=remote-build-a, transport=direct, reason=SIDE_EFFECTING_TASK, context_hash=abc123def456, runtime=containerized, runtime_engine=podman)
```

Key fields:

- `placement=` local or remote placement mode.
- `remote_node=` chosen remote node id, or `none`.
- `transport=` transport class (`direct`, `tor`, or `none`).
- `reason=` policy or placement reason.
- `context_hash=` workspace context manifest hash used for remote decisions.
- `runtime=` runtime kind resolved for remote execution.
- `runtime_engine=` concrete runtime engine when applicable.

## Makefile Mode

`tak make <goal>` does not load or require `TASKS.py`. Tak reads the same default file name order
as GNU Make (`GNUmakefile`, `makefile`, then `Makefile`), resolves a literal target header, and
executes one opaque `make <goal>` command by default. Without a parallel annotation, Make—not
Tak—expands and schedules that goal's prerequisites exactly as before.

File-wide defaults avoid repeating the same execution settings above every goal. Prefix each
default key with `default.`:

```make
# tak: default.execution=remote
# tak: default.container-image=ghcr.io/acme/build:latest

build:
	./scripts/build.sh

test: build
	./scripts/test.sh
```

Contiguous comments immediately above a goal can select placement and a container:

```make
# tak: execution=remote
# tak: container-dockerfile=docker/test.Dockerfile
# tak: container-build-context=.
test: build
	./scripts/test.sh
```

Supported goal keys are `execution=local|remote`, `container-image=<reference>`,
`container-dockerfile=<path>`, `container-build-context=<path>`,
`parallel=<goal,goal,...>`, and `parallel-output=live|grouped`. All except `parallel` may use the
`default.` prefix. Goal settings override compatible default fields, while unmentioned settings
are inherited. A goal can override only the build context while inheriting a default Dockerfile.
Selecting a goal image replaces a default Dockerfile and context, while selecting a goal Dockerfile
replaces a default image.

An aggregate may promote two or more direct phony prerequisites into Tak's execution graph:

```make
.PHONY: check lint test build

# tak: parallel=lint,test,build
# tak: parallel-output=grouped
check: lint test build
	./scripts/report-success.sh

lint:
	./scripts/lint.sh
test:
	./scripts/test.sh
build:
	./scripts/build.sh
```

Tak starts every ready promoted goal concurrently, waits for them, then invokes GNU Make for the
aggregate with each completed child passed through `--assume-old`. This lets the aggregate recipe
run without repeating phony children; prerequisites not named by `parallel` remain Make-owned.
Groups may be nested. Common execution/container annotations on an aggregate flow recursively to
its promoted children, and a child's own annotations override inherited values. CLI execution and
`--parallel-output` flags override the entire graph.

Every annotated group and listed member must be declared in `.PHONY`, members must be unique direct
literal prerequisites, and at least two members are required. Tak rejects parallel cycles, dynamic
or continued prerequisite declarations, and shared goals that inherit conflicting execution
settings. GNU Make is required for annotated parallel groups.

Live output is the default and prefixes each logical line with `[goal]`. Grouped output holds each
goal's lines until it finishes and then emits them contiguously with the same prefix. After a child
failure, unrelated branches finish, dependent aggregates are skipped, and Tak returns the first
failed goal in recursive left-to-right annotation order.

The complete precedence order is command-line flags, target annotations, enclosing aggregate
annotations, global defaults, then the implicit local-host fallback. The command accepts the same
`--local`, `--local-no-container`,
`--remote`, `--container`, `--container-image`, `--container-dockerfile`, and
`--container-build-context` overrides as `tak exec`. An inherited container remains selected when a
goal only changes `execution`; use `--local-no-container` when an invocation must explicitly ignore
all authored container configuration.

When no applicable Makefile or command-line execution configuration exists, Tak writes an `info:`
notice to stderr before running locally outside a container. The notice points to global defaults,
goal annotations, and CLI overrides that enable remote execution; Make's stdout remains unchanged.

The annotation reader intentionally supports only literal single-target `target: prerequisites`
headers. It does not interpret includes, expanded target names, generated rules, multi-target rules,
pattern rules, target-specific variable assignments, or double-colon rules. Unsupported annotated declarations fail clearly instead of
silently selecting the wrong runtime. Remote Make runs stream output and status back, but generated
files are not materialized locally because `tak make` has no declared output paths yet. Consequently,
parallel remote goals must be independent: one promoted goal cannot consume files generated by
another promoted goal.

## Quickstart

For the current ergonomics story and distributed execution roadmap, see [Ergonomics and Distributed Execution Phases](docs/ergonomics-and-distribution-phases.md).

1. Optional but recommended for remote execution.

On the agent machine:

```bash
takd init
takd serve
takd status
takd token show --qr --wait
takd token show --words --wait
```

On the client machine:

```bash
tak remote scan
tak remote add 'SECRET_TAKD_INVITE'
# or:
tak remote add --words SECRET_WORD_01 ... SECRET_WORD_19
tak remote status
```

Direct transport examples need matching agent settings, for example `takd init --transport direct --base-url http://127.0.0.1:0 --pool build` for build pools or `--pool test` for test pools.

Containerized tasks can point at either a prebuilt image or a checked-in Dockerfile:

```python
LOCAL = Execution.Local(container=Container.Dockerfile(path("docker/Dockerfile")))

REMOTE = Execution.Remote(
    pool="build",
    required_tags=["builder"],
    required_capabilities=["linux"],
    container=Container.Image("alpine:3.20"),
)
```

Remote workers own the defaults for container resources. A remote container without authored
limits reserves 4 logical CPU cores and 8192 MiB by default, clamped to the worker's logical core
count and safe RAM admission capacity. CPU is enforced as a container quota and also caps common
test/codegen thread pools. Memory is an admission reservation only: Tak does not install a Docker
memory cap and retains its pause/backpressure policy instead of killing work under pressure.

Operators can change the node defaults with `TAKD_DEFAULT_CONTAINER_CPU_CORES` and
`TAKD_DEFAULT_CONTAINER_MEMORY_MB`. Aggregate reservations use strict 1x admission by default;
`TAKD_ADMISSION_OVERSUBSCRIBE_X` is an explicit opt-in to oversubscription. Authored
`Container.Resources(...)` values are preserved and evaluated by admission without clamping.
Unsized local containers, including CLI-created local runtimes, remain unaffected.

Remote infrastructure failures (including exit 137 and exhausted worker/container transport
recovery) retry once on each distinct eligible worker without consuming the task's authored retry
attempts. Ordinary nonzero task exits and authored timeouts remain terminal under the task's normal
retry policy, and cancellation is never failed over.

Runtime model:

- local host execution
- local containerized execution
- remote containerized execution

Use `--local-no-container` when a task has a remote/container fallback policy but you want the local host path explicitly. Use `--local --container` for local containerized execution, and use `--remote` for remote containerized execution.

### Needs, Leases, And Cascades

`needs` are resource requests, not task dependencies. Dependencies decide which tasks must finish before another task can run. `needs` tell Tak what shared capacity a task wants before it starts, such as one exclusive UI lock, two CPU slots from a machine pool, or one token from a rate limiter.

A lease is Tak's permission slip from the local daemon. If a lease socket is configured, Tak asks for the lease before local host work, local container work, remote work, or fused container work starts. Two tasks that both need the same exclusive lock wait their turn instead of running together.

For example:

```python
test_ui = task(
    "test-ui",
    needs=[need("ui_lock", 1, scope=Scope.Machine)],
    steps=[cmd("sh", "-c", "echo run ui tests")],
)

SPEC = module_spec(
    tasks=[test_ui],
    limiters=[lock("ui_lock", scope=Scope.Machine)],
)
SPEC
```

Remote execution still reports submitted `needs` to the remote `takd` process so status views can show what a job asked for. The limit is enforced by the client-side lease acquired before the remote submit.

A cascaded container session can run a dependency chain inside one per-run container instead of launching one container per task. Shared dependencies are allowed when the roots use the same execution and session. If different cascades try to pull the same dependency into different executions, Tak rejects the run before starting work.

When a fused container cascade has members with `needs`, Tak merges those requests and acquires one lease before launching the fused run. Local fused execution reuses that outer lease; it does not acquire duplicate per-member leases.

For Tor onboarding, `takd token show --wait` now waits until the agent-side `takd` process has verified that its onion service answers `/v1/node/info` through Tor. `tak remote add` still performs its own probe from the client machine, and another machine can still need a short additional propagation window before the onion endpoint is reachable there.

If you need to type the invite instead of scanning it, use `takd token show --words --wait`. The emitted 19-word phrase encodes the Tor v3 onion host directly and ends with a checksum word, so `tak remote add --words ...` can reject typos before any network probe.

If `tak remote add` still times out probing a new onion endpoint, inspect the server directly:

```bash
takd status
takd logs --lines 50
```

2. Change into a project directory that contains `TASKS.py`, then explore and run a target:

```bash
tak list
tak tree
tak explain //apps/web:test_ui
tak graph //apps/web:test_ui --format dot
tak run //apps/web:test_ui -j 4 --keep-going
```

For an existing Make project with no `TASKS.py`, run a goal directly:

```bash
tak make test
```

Workspace rules:

- Workspace graph commands load only the current directory's `TASKS.py`; `tak make` uses the
  current directory's default Makefile instead.
- Tak never widens scope by scanning parent or child directories implicitly.
- Multi-package projects compose extra modules explicitly with `module_spec(includes=[path("apps/web"), ...])`.
- At a workspace root, `tak run hello` is shorthand for `tak run //:hello`.
- `tak run .` is not shorthand for "this project"; use labels returned by `tak list`.

## Copy-Paste TASKS.py Starter

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
    limiters=[lock("ci_lock", scope=Scope.Machine)],
)
SPEC
```

## Crate Map

- `crates/tak-core`: canonical model types, labels, DAG planner.
- `crates/tak-make`: injected Makefile reader, literal annotation resolver, and goal execution use case.
- `crates/tak-loader`: `TASKS.py` discovery, evaluation, and merge.
- `crates/tak-exec`: runtime executor, retry/timeout handling, and remote placement.
- `crates/takd`: standalone execution agent and sqlite-backed remote submit store.
- `crates/tak`: CLI contracts and interactive web graph serving.

## Installation

Install the latest release for your platform:

```bash
curl -fsSL https://raw.githubusercontent.com/gu1p/tak/main/get-tak.sh | bash
curl -fsSL https://raw.githubusercontent.com/gu1p/tak/main/get-takd.sh | bash
```

Install behavior:

- Downloads latest public release asset for macOS/Linux (`x86_64` + `aarch64`).
- Installs `tak` and `takd` to `~/.local/bin` by default.
- Adds the install directory to your shell startup file when it is not already on `PATH`.
- `get-takd.sh` installs and bootstraps the standalone `takd` Tor agent service.
- `get-takd.sh` prints install milestones, selected Tor readiness highlights, and a terminal QR code after the agent is ready.
- Full `takd` service logs stay on disk; use `takd logs --all` or `takd logs --lines 200` to inspect them.
- Supports overrides:
  - `TAK_VERSION` to pin a release tag.
  - `TAK_INSTALL_DIR` to change install destination.
  - `TAK_REPO` to install from a different repository.
  - `TAKD_INSTALLER_VERBOSE=1` to stream deep readiness diagnostics during installation.
  - `TAKD_TRANSPORT`, `TAKD_BASE_URL`, `TAKD_POOLS`, `TAKD_TAGS`, and `TAKD_CAPABILITIES` to customize the initial agent config.

## Working on Tak Itself

Inside this repo, use the system `tak` already on `PATH`.

If you need to bootstrap a fresh machine from this checkout, run `./get-tak.sh` once and then use `tak run ...` from the installed binary. GitHub Actions in this repo follow the same bootstrap path.

For local source installs, `./install-locally.sh` builds with stable Rust. If `cargo +stable` is unavailable and your active Cargo toolchain is nightly-only, the script stops with an explicit stable-toolchain error instead of attempting a nightly build.

This repo bootstraps from the latest released Tak, so root `TASKS.py` and contributor guidance must stay compatible with the released CLI surface.

## Quality Gates

```bash
tak run //:check
tak run //:coverage
```

- `tak run //:check` runs formatting, clippy, tests, doctests, and docs-policy contracts.
- `tak run //:coverage` writes the LCOV report to `.tmp/coverage/lcov.info`.

## Documentation Map

- Agent authoring bundle: `tak docs dump`
- Phased ergonomics and distribution guide: [`docs/ergonomics-and-distribution-phases.md`](docs/ergonomics-and-distribution-phases.md)
- System overview: [`ARCHITECTURE.md`](ARCHITECTURE.md)
- Core internals: [`crates/tak-core/ARCHITECTURE.md`](crates/tak-core/ARCHITECTURE.md)
- Loader internals: [`crates/tak-loader/ARCHITECTURE.md`](crates/tak-loader/ARCHITECTURE.md)
- Executor internals: [`crates/tak-exec/ARCHITECTURE.md`](crates/tak-exec/ARCHITECTURE.md)
- Daemon internals: [`crates/takd/ARCHITECTURE.md`](crates/takd/ARCHITECTURE.md)
- CLI contracts: [`crates/tak/ARCHITECTURE.md`](crates/tak/ARCHITECTURE.md)
