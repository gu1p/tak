# Tak Examples Matrix

This directory is a contract-backed examples catalog for Tak features and failure modes.

Source of truth:

- machine-readable catalog: [`catalog.toml`](catalog.toml)
- executable validation: `crates/tak/tests/examples_matrix_contract.rs`

## How to Read an Example

Each example folder contains:

- `TASKS.py` (root): project identity (`module_spec(project_id=...)`)
- root `TASKS.py`: local module definitions
- optional nested `TASKS.py`: recursive packages
- optional scripts consumed by task steps
- `README.md`: scenario, expected command answers, expected outputs

## Command Workflow

For any example:

```bash
tak list
tak explain <target>
tak graph <target> --format dot
tak web <target>
tak run <target>
```

If example requires daemon-backed `needs`:

```bash
tak daemon start
```

## Coverage Matrix

| Tier | IDs | Focus |
|---|---|---|
| small | 01-10 | isolated DSL/runtime behaviors |
| medium | 11-20 | multi-feature scenarios + scoped coordination |
| large | 21-24 | recursive multi-package topologies and realistic pipelines |

Feature areas covered:

- label resolution and dependency ordering
- command/script execution, env and cwd behavior
- defaults inheritance and exclusion patterns
- retry/backoff and timeout behavior
- machine locks, resource pools, queue disciplines
- scope-aware limiter keys (machine/user/project/worktree)
- daemon contention and lease scheduling
- recursive monorepo and polyglot release chains

## Catalog Contract Fields

Each `[[example]]` entry defines:

- `name`: relative folder under `examples/`
- `run_target`: target used for `tak run`
- `explain_target`: target used for explain/graph checks
- `expect_success`: expected run exit outcome
- `requires_daemon`: whether daemon is required
- `check_files`: output files that must exist on successful run

## Validation

Run only examples contract:

```bash
cargo test -p tak --test examples_matrix_contract
```

Run full project gate:

```bash
make check
```
