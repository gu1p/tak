# Tak

Tak is a local task orchestrator for recursive monorepos. It loads distributed `TASKS.py` definitions, resolves them into one validated dependency graph, executes targets in dependency order, and optionally coordinates shared machine resources through `takd`.

## What Problem It Solves

- teams define tasks near code (`TASKS.py` in many folders)
- orchestration still works as one global graph
- resource contention (CPU, RAM, locks, queues) can be coordinated centrally
- retries/timeouts are standardized instead of copy-pasted in scripts

## Core Capabilities

- Recursive workspace loading with gitignore-aware discovery
- Strict label parsing (`//pkg:task`, `:task`)
- Topological execution with missing-dep/cycle validation
- Step execution for commands and scripts
- Retry policy with fixed or exponential-jitter backoff
- Optional daemon lease coordination for `needs`
- SQLite-backed daemon recovery/history

## Crate Map

- `crates/tak-core`: canonical model types, labels, DAG planner
- `crates/tak-loader`: `TASKS.py` discovery/evaluation/merge
- `crates/tak-exec`: runtime executor and lease-client integration
- `crates/takd`: unix-socket daemon + lease manager + sqlite persistence
- `crates/tak`: CLI surface, output contracts, dispatch

## CLI Quick Reference

- `tak list`
  - answers: "what tasks exist?"
  - output: one fully-qualified label per line
- `tak explain <label>`
  - answers: "what does this task depend on / contain?"
  - output: label, deps, steps, needs, timeout, retry attempts
- `tak graph [label] --format dot`
  - answers: "what graph should be visualized?"
  - output: DOT graph
- `tak web [label]`
  - answers: "show me this graph interactively in a browser"
  - output: local URL + embedded web graph server until `Ctrl+C`
- `tak run <label...>`
  - answers: "did execution succeed and how many attempts were needed?"
  - output: one status line per executed task
- `tak status`
  - answers: "what is daemon lease pressure right now?"
  - output: active leases, pending requests, limiter usage rows
- `tak daemon start|status`
  - answers: "start daemon" / "is daemon responsive"

For full command contracts, see [`crates/tak/ARCHITECTURE.md`](crates/tak/ARCHITECTURE.md).

## Quickstart

1. Start daemon in a separate terminal:

```bash
tak daemon start
```

2. In a workspace, inspect and run targets:

```bash
tak list
tak explain //apps/web:test_ui
tak graph //apps/web:test_ui --format dot
tak run //apps/web:test_ui
```

## Installation

Install the latest release for your platform:

```bash
curl -fsSL https://raw.githubusercontent.com/gu1p/tak/main/get-tak.sh | bash
```

Install behavior:

- downloads the newest release asset for macOS/Linux (`x86_64` + `aarch64`)
- installs `tak` and `takd` to `~/.local/bin` by default
- supports overrides:
  - `TAK_VERSION` to pin a release tag
  - `TAK_INSTALL_DIR` to change install destination
  - `TAK_REPO` to install from a different repository
  - `GH_TOKEN` or `GITHUB_TOKEN` for private repository access

Current repository visibility is private while polishing; unauthenticated curl installs will work once the repository is public.

## CI/CD and Releases

- CI workflow (`.github/workflows/ci.yml`) runs on PRs and pushes to `main`:
  - formatting (`cargo fmt --check`)
  - linting (`cargo clippy -- -D warnings`)
  - tests (`cargo test --workspace`)
  - doctests (`cargo test --workspace --doc`)
  - docs policy contract (`cargo test -p tak --test doctest_contract`)
  - coverage gate (`cargo llvm-cov --fail-under-lines 75`)
- Release workflow (`.github/workflows/release.yml`) runs after successful CI on `main`:
  - auto-generates SemVer tags (`vX.Y.Z`)
  - builds `tak` and `takd` for:
    - `x86_64-unknown-linux-musl`
    - `aarch64-unknown-linux-musl`
    - `x86_64-apple-darwin`
    - `aarch64-apple-darwin`
  - creates/updates GitHub Releases with binaries and `.sha256` checksums

## Versioning

- `tak --version` is wired to `TAK_VERSION` at build time.
- Release builds inject the value from the generated Git tag (`vX.Y.Z` -> `X.Y.Z`).
- Local developer builds fall back to the crate package version.

## Examples

- Full matrix in [`examples/catalog.toml`](examples/catalog.toml)
- Human index in [`examples/README.md`](examples/README.md)
- Contract test runs every entry: `crates/tak/tests/examples_matrix_contract.rs`

## Quality Gates

```bash
make check
```

`make check` runs:

- formatting check
- clippy (warnings denied)
- workspace tests
- doctests for all crates
- docs policy contract (`doctest_contract`)

## Documentation Map

- System overview: [`ARCHITECTURE.md`](ARCHITECTURE.md)
- Core internals: [`crates/tak-core/ARCHITECTURE.md`](crates/tak-core/ARCHITECTURE.md)
- Loader internals: [`crates/tak-loader/ARCHITECTURE.md`](crates/tak-loader/ARCHITECTURE.md)
- Executor internals: [`crates/tak-exec/ARCHITECTURE.md`](crates/tak-exec/ARCHITECTURE.md)
- Daemon internals: [`crates/takd/ARCHITECTURE.md`](crates/takd/ARCHITECTURE.md)
- CLI contracts: [`crates/tak/ARCHITECTURE.md`](crates/tak/ARCHITECTURE.md)
