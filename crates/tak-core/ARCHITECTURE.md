# tak-core Architecture

## Purpose

`tak-core` is the shared domain layer. It contains model types and deterministic algorithms used by loader, executor, daemon, and CLI crates.

Most code here is pure model/planner logic. The crate also owns narrowly scoped shared path and
inventory helpers so `tak`, `tak-exec`, and `takd` agree on runtime socket and `remotes.toml`
locations.

## Responsibilities

- Canonical model definitions (`TaskLabel`, task specs, limiters, retries, workspace spec).
- Label parsing and validation helpers.
- DAG topological planning with cycle/missing-node detection.
- Shared runtime path helpers such as the default local daemon unix socket path.
- Shared remote inventory parsing for `$XDG_CONFIG_HOME/tak/remotes.toml`.

## Public Surface

- `label::parse_label(raw, current_package) -> Result<TaskLabel, LabelError>`
- `label::normalize_package(package) -> String`
- `planner::topo_sort(dep_map) -> Result<Vec<TaskLabel>>`
- `model::*` canonical structs/enums used across all crates
- `runtime_paths::default_daemon_socket_path() -> PathBuf`
- `remote_inventory::{RemoteInventory, RemoteRecord}` plus load/save helpers

## Inputs and Outputs

- Input: plain strings/maps/vectors.
- Output: typed domain values + explicit errors.
- Side effects: pure model/planner APIs have none; inventory load/save helpers perform explicit
  filesystem reads/writes for the shared `remotes.toml` contract.

## Invariants

- Labels must be fully qualified after parsing.
- Package labels must start with `//`.
- Task names must be non-empty.
- Topological planner returns dependency-first order.
- Cyclic graphs fail deterministically.

## Error Model

- `LabelError`: format/package/name issues.
- `anyhow::Error` from planner for missing deps/cycles.

## Design Notes

- Kept dependency-light to avoid cross-crate coupling; shared IO helpers are intentionally small
  and configuration-specific.
- Structures are serde-compatible for loader and daemon protocol composition.
- Deterministic behavior supports stable tests and reproducible execution plans.

## Main Files

- `src/model.rs`: shared type system for task/workspace/limiter/retry entities.
- `src/label.rs`: parser and validation for task labels.
- `src/planner.rs`: topological ordering and graph validation.
- `src/runtime_paths.rs`: shared local daemon socket path resolution.
- `src/remote_inventory.rs`: shared `remotes.toml` schema, parsing, and persistence.
