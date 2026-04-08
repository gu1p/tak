# Tak

Tak is a task orchestrator for recursive monorepos. It loads distributed `TASKS.py` files, builds one validated dependency graph, and executes local and remote work with consistent retry, timeout, and resource-coordination behavior.

## Why Teams Use Tak

- Keep task definitions close to code while still running one global graph.
- Coordinate shared machine resources (`cpu`, `ram`, locks, queues, rate limits, process caps) without custom glue scripts.
- Standardize execution behavior (timeouts, retries, remote placement, artifact sync) across local dev and CI.
- Keep failure diagnostics actionable with deterministic outputs and logs.

## Core Capabilities

- Recursive workspace loading with gitignore-aware discovery.
- Strict label parsing for absolute and relative task references.
- DAG validation (missing dependency and cycle detection) before execution.
- Command and script step execution with explicit `cwd` and `env` control.
- Retry policies with fixed or exponential-jitter backoff.
- Timeout controls per task.
- Client-side lease coordination for `needs` (resource/lock/rate/process/queue semantics).
- Remote execution with direct or Tor transport plus artifact roundtrip.
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

## CLI Quick Reference

- `tak list`
  - Enumerate fully-qualified task labels.
- `tak tree`
  - Render tasks as a tree for quick topology inspection.
- `tak explain <label>`
  - Show task composition (`deps`, `steps`, `needs`, timeout, retry).
- `tak graph [label] --format dot`
  - Print DOT graph for Graphviz or pipeline tooling.
- `tak web [label]`
  - Serve an interactive dependency graph UI locally.
- `tak run <label...>`
  - Execute targets and dependencies.
- `tak run <label...> -j <N> --keep-going`
  - Configure parallelism and continue with independent work after failures.
- `--keep-going`
  - Continue independent tasks even after one target fails.
- `tak status`
  - Report coordination status when supported; the current client-only build returns an unsupported error.
- `tak remote add <token>`
  - Import a `takd` agent token into local client config.
- `tak remote list`
  - Show configured remote agents in client priority order.
- `takd init`
  - Create Tor-first agent identity and hidden-service runtime state.
- `takd serve`
  - Start the standalone execution agent service and publish its hidden-service token when ready.
- `takd token show`
  - Reprint the persisted onboarding token, or wait until it is advertised with `--wait`.

## Run Output Signals

`tak run` prints one summary line per executed task. The line is intentionally rich so you can debug placement decisions quickly.

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

1. Optional but recommended for remote execution:

```bash
takd init
takd serve
tak remote add "$(takd token show --wait)"
```

Direct transport examples need matching agent settings, for example `takd init --transport direct --base-url http://127.0.0.1:0 --pool build` for build pools or `--pool test` for test pools.

For Tor onboarding, `tak remote add` waits briefly for a fresh onion service to answer `/v1/node/info`. `takd token show --wait` only guarantees that the onboarding token has been published locally; it does not verify that another machine can already reach the onion service.

2. Explore and run a target:

```bash
tak list
tak tree
tak explain //apps/web:test_ui
tak graph //apps/web:test_ui --format dot
tak run //apps/web:test_ui -j 4 --keep-going
```

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
    tasks=[build, test],
    limiters=[lock("ci_lock", scope=MACHINE)],
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

- Downloads latest release asset for macOS/Linux (`x86_64` + `aarch64`).
- Installs `tak` and `takd` to `~/.local/bin` by default.
- `get-takd.sh` installs and bootstraps the standalone `takd` Tor agent service.
- Supports overrides:
  - `TAK_VERSION` to pin a release tag.
  - `TAK_INSTALL_DIR` to change install destination.
  - `TAK_REPO` to install from a different repository.
  - `GH_TOKEN` or `GITHUB_TOKEN` for private repository access.
  - `TAKD_TRANSPORT`, `TAKD_BASE_URL`, `TAKD_POOLS`, `TAKD_TAGS`, and `TAKD_CAPABILITIES` to customize the initial agent config.

## Quality Gates

```bash
make check
```

`make check` runs formatting, clippy, tests, doctests, and docs-policy contracts.

## Documentation Map

- System overview: [`ARCHITECTURE.md`](ARCHITECTURE.md)
- Core internals: [`crates/tak-core/ARCHITECTURE.md`](crates/tak-core/ARCHITECTURE.md)
- Loader internals: [`crates/tak-loader/ARCHITECTURE.md`](crates/tak-loader/ARCHITECTURE.md)
- Executor internals: [`crates/tak-exec/ARCHITECTURE.md`](crates/tak-exec/ARCHITECTURE.md)
- Daemon internals: [`crates/takd/ARCHITECTURE.md`](crates/takd/ARCHITECTURE.md)
- CLI contracts: [`crates/tak/ARCHITECTURE.md`](crates/tak/ARCHITECTURE.md)
