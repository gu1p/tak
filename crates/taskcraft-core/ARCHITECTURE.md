# taskcraft-core Architecture

## Purpose

`taskcraft-core` is the shared domain layer. It contains model types and deterministic algorithms used by loader, executor, daemon, and CLI crates.

No IO or environment interaction should live here.

## Responsibilities

- Canonical model definitions (`TaskLabel`, task specs, limiters, retries, workspace spec).
- Label parsing and validation helpers.
- DAG topological planning with cycle/missing-node detection.

## Public Surface

- `label::parse_label(raw, current_package) -> Result<TaskLabel, LabelError>`
- `label::normalize_package(package) -> String`
- `planner::topo_sort(dep_map) -> Result<Vec<TaskLabel>>`
- `model::*` canonical structs/enums used across all crates

## Inputs and Outputs

- Input: plain strings/maps/vectors.
- Output: typed domain values + explicit errors.
- Side effects: none.

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

- Kept dependency-light to avoid cross-crate coupling.
- Structures are serde-compatible for loader and daemon protocol composition.
- Deterministic behavior supports stable tests and reproducible execution plans.

## Main Files

- `src/model.rs`: shared type system for task/workspace/limiter/retry entities.
- `src/label.rs`: parser and validation for task labels.
- `src/planner.rs`: topological ordering and graph validation.
