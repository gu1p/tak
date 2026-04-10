# Tak Examples Matrix

This directory is an executable catalog of Tak behavior. The examples are tested as contracts, not only as documentation snippets.

Source of truth:

- Machine-readable catalog: [`catalog.toml`](catalog.toml)
- Executable validation: `crates/tak/tests/examples_matrix_contract.rs`

## Start Here: Hero Path

Use these 8 examples first. They are the fastest path to using Tak in real projects.

| Example | Why you should read it | Primary capabilities | Run target |
|---|---|---|---|
| [`small/01_hello_single_task`](small/01_hello_single_task/README.md) | Smallest working task graph | basic `task`, `cmd`, output artifact | `//:hello` |
| [`small/04_cmd_with_env_and_cwd`](small/04_cmd_with_env_and_cwd/README.md) | Reliable shell behavior | step-level `cwd` and `env` control | `//:env_cmd` |
| [`small/08_retry_fixed_fail_once`](small/08_retry_fixed_fail_once/README.md) | Stable flaky-task handling | retry attempts, `on_exit`, fixed backoff | `//:flaky_fixed` |
| [`medium/11_machine_lock_shared_ui`](medium/11_machine_lock_shared_ui/README.md) | Safe shared resources | `lock`, `need`, machine scope coordination | `//:ui_test` |
| [`medium/18_multi_package_monorepo`](medium/18_multi_package_monorepo/README.md) | Real monorepo composition | explicit `includes=[...]`, cross-package deps | `//apps/web:all` |
| [`large/24_full_feature_matrix_end_to_end`](large/24_full_feature_matrix_end_to_end/README.md) | Combined high-load flow | limiters, queues, defaults, retries, scripts | `//apps/qa:release` |
| [`large/25_remote_direct_build_and_artifact_roundtrip`](large/25_remote_direct_build_and_artifact_roundtrip/README.md) | Practical remote build | `RemoteOnly`, artifact sync, local verify | `//services/api:release` |
| [`large/28_hybrid_local_remote_test_suite_failure_with_logs`](large/28_hybrid_local_remote_test_suite_failure_with_logs/README.md) | Failure diagnostics at scale | hybrid local+remote, non-zero remote suite, log retention | `//apps/web:remote_suite` |

Need more context before jumping into examples?

- One phased guide: [`../docs/ergonomics-and-distribution-phases.md`](../docs/ergonomics-and-distribution-phases.md)
  - Explains what Tak already has today, what should come next, and the longer-term distributed execution direction.

## Choose by Capability

- Local baseline task execution: `small/01`
- Step environment and working directory control: `small/04`
- Retries/backoff behavior: `small/08`, `small/09`
- Daemon coordination and scoped needs: `medium/11` through `medium/17`
- Explicit multi-package graphs: `medium/18`, `large/21`
- Full composition with queues + defaults + scripts: `large/24`
- Remote artifact workflows: `large/25`, `large/26`
- Hybrid local+remote test suites and diagnostics: `large/27`, `large/28`
- Transport-agnostic remote container execution and heavy log streaming: `large/29`

## Standard Command Workflow

For any example:

```bash
tak list
tak explain <target>
tak graph <target> --format dot
tak web <target>
tak run <target>
```

All commands are meant to run from the example directory itself. Tak loads only that directory's `TASKS.py`; multi-package examples bring in other modules through explicit `includes=[...]`.

If the example target lives at the root package, a bare task name also works. Example: `tak run hello` is shorthand for `tak run //:hello`.

If the example uses remote execution, import and run a `takd` agent first:

```bash
takd init
takd serve
takd status
tak remote add "$(takd token show --wait)"
```

Direct examples need matching init flags, for example `takd init --transport direct --base-url http://127.0.0.1:0 --pool build` or `--pool test`.

If remote onboarding fails, inspect the remote server with `takd status` and `takd logs --lines 50`.

## Reference Scenarios (Complete Matrix)

The full matrix remains important for regression and feature parity checks.

| Tier | IDs | Focus |
|---|---|---|
| small | 01-10 | isolated DSL/runtime behaviors |
| medium | 11-20 | multi-feature scenarios + scoped coordination |
| large | 21-29 | explicit include graphs, remote execution, Tor transport, hybrid pipelines |

To inspect every example entry and its contracts, open [`catalog.toml`](catalog.toml).

## Catalog Contract Fields

Each `[[example]]` entry defines:

- `name`: relative folder under `examples/`
- `run_target`: target used for `tak run`
- `explain_target`: target used for explain/graph checks
- `expect_success`: expected run exit outcome
- `requires_daemon`: whether daemon is required
- `remote_fixture` (optional): deterministic remote fixture (`direct_http` or `tor_onion_http`)
- `simulate_container_runtime` (optional): enable test-only container probe simulation for remote fixtures
- `expect_stdout_contains` (optional): required substrings in run stdout
- `expect_stderr_contains` (optional): required substrings in run stderr
- `check_files`: output files that must exist after run
- `check_file_contains` (optional): content assertions for expected output files

## Validation

Run only examples contract:

```bash
cargo test -p tak --test examples_matrix_contract
```

Run full project gate:

```bash
make check
```
