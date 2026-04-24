# Tak

Tak is a task orchestrator for project-local `TASKS.py` workspaces. It loads the current directory's `TASKS.py`, follows explicit `module_spec(includes=[...])` links, builds one validated dependency graph, and executes local host work, local containerized work, and remote containerized work with consistent retry, timeout, and resource-coordination behavior.

## Why Teams Use Tak

- Keep task definitions close to code while still running one global graph.
- Coordinate shared machine resources (`cpu`, `ram`, locks, queues, rate limits, process caps) without custom glue scripts.
- Standardize execution behavior (timeouts, retries, remote placement, artifact sync) across local dev and CI.
- Keep failure diagnostics actionable with deterministic outputs and logs.

## Core Capabilities

- Current-directory workspace loading with explicit `module_spec(includes=[...])` composition.
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
- `tak run <label...>`
  - Execute targets and dependencies.
- `tak run hello`
  - At a workspace root, bare task names are shorthand for root-package labels such as `//:hello`.
- `tak run <label...> -j <N> --keep-going`
  - Configure parallelism and continue with independent work after failures.
- `tak run //:check`
  - Run the repo-owned quality gate declared in `TASKS.py`.
- `tak run .`
  - Invalid input. Use `tak list` first, then pass a real label such as `//:task` or `//pkg:task`.
- `--keep-going`
  - Continue independent tasks even after one target fails.
- `tak status`
  - Report coordination status when supported; the current client-only build returns an unsupported error.
- `tak remote add <token>`
  - Import a `takd` agent token into local client config.
- `tak remote add`
  - Open an interactive terminal flow for adding a remote from words, a token, or a Tor `.onion` location.
- `tak remote add --words <word>...`
  - Import a Tor v3 `takd` invite from the 19-word manual-entry phrase emitted by `takd token show --words`.
- `tak remote add --words`
  - Open the interactive word-entry flow directly.
- `tak remote scan`
  - Pick a camera, preview its feed in the terminal, and add a remote from a scanned QR token.
- `tak remote list`
  - Show configured remote agents in client priority order.
- `tak remote status`
  - Show running jobs plus CPU, RAM, and storage usage for configured remote agents.
- `tak remote status --watch --interval-ms <N>`
  - Refresh remote node status continuously in-place.
- `takd init`
  - Create Tor-first agent identity and hidden-service runtime state.
- `takd serve`
  - Start the standalone execution agent service and publish its hidden-service token when ready.
- `takd status`
  - Show the agent transport/readiness plus the resolved `service.log` path and whether it exists yet.
- `takd logs`
  - Print the most recent server-side `takd` log lines from the agent state directory.
- `takd token show`
  - Reprint the persisted onboarding token, or wait until it is advertised with `--wait`.
- `takd token show --words`
  - Print the 19-word Tor v3 onboarding phrase for manual typing.
- `takd token show --words-table`
  - Print the same Tor v3 onboarding phrase as numbered cells for human copying.
- `takd token show --qr`
  - Render the onboarding token as a terminal QR code plus the exact `tak remote add '...'` command, and show numbered word cells when the invite targets a real Tor v3 onion host.

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

## Quickstart

For the current ergonomics story and distributed execution roadmap, see [Ergonomics and Distributed Execution Phases](docs/ergonomics-and-distribution-phases.md).

1. Optional but recommended for remote execution:

```bash
takd init
takd serve
takd status
tak remote add "$(takd token show --wait)"
tak remote add --words "$(takd token show --words --wait)"
tak remote scan
tak remote status
```

Direct transport examples need matching agent settings, for example `takd init --transport direct --base-url http://127.0.0.1:0 --pool build` for build pools or `--pool test` for test pools.

Containerized tasks can point at either a prebuilt image or a checked-in Dockerfile:

```python
LOCAL = Execution.Local(runtime=Runtime.Dockerfile(path("docker/Dockerfile")))

REMOTE = Execution.Remote(
    pool="build",
    required_tags=["builder"],
    required_capabilities=["linux"],
    runtime=Runtime.Image("alpine:3.20"),
)
```

Runtime model:

- local host execution
- local containerized execution
- remote containerized execution

For Tor onboarding, `takd token show --wait` now waits until the local `takd` process has verified that its onion service answers `/v1/node/info` through Tor. `tak remote add` still performs its own probe, and another machine can still need a short additional propagation window before the onion endpoint is reachable there.

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

Workspace rules:

- Tak loads only the current directory's `TASKS.py`.
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
- `get-takd.sh` installs and bootstraps the standalone `takd` Tor agent service.
- `get-takd.sh` prints a terminal QR code for the onboarding token after the agent is ready.
- Supports overrides:
  - `TAK_VERSION` to pin a release tag.
  - `TAK_INSTALL_DIR` to change install destination.
  - `TAK_REPO` to install from a different repository.
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
