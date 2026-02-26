# taskcraft-loader Architecture

## Purpose

`taskcraft-loader` transforms a recursive set of `TASKS.py` files into one validated `WorkspaceSpec`.

It is responsible for discovery, evaluation, conversion, merge, and graph-level validation before execution begins.

## Pipeline

```mermaid
flowchart LR
    Discover[Find TASKS.py files] --> Eval[Monty evaluation]
    Eval --> Convert[Monty object -> strict JSON]
    Convert --> Decode[JSON -> ModuleSpec]
    Decode --> Merge[Resolve labels/defaults/scopes]
    Merge --> Validate[Unknown deps + DAG validation]
    Validate --> Workspace[WorkspaceSpec]
```

## Responsibilities

- Detect workspace root (`taskcraft.toml`, `.git`, fallback cwd).
- Discover all `TASKS.py` files using gitignore-aware traversal.
- Execute each file with DSL prelude under bounded Monty limits.
- Convert Monty values into strict JSON-compatible structures.
- Deserialize into `ModuleSpec` and merge into global registries.
- Resolve limiter scope keys and task labels.
- Validate dependencies and acyclic graph property.

## Key Contracts

- Every merged task label is absolute and unique.
- Dependencies must reference existing tasks.
- Module defaults apply consistently when task-local values are absent.
- Scope keys are derived from scope type (`machine/user/project/worktree`).

## Failure Classes

- root detection/path canonicalization errors
- syntax/runtime/type-checking failures during Monty eval
- object conversion failures for unsupported runtime values
- parse failures for module schema
- duplicate/conflicting definitions
- unknown dependencies or cycles

## Main Functions

- `detect_workspace_root`
- `discover_tasks_files`
- `load_workspace`
- `eval_module_spec`
- `merge_module`

## Main Files

- `src/lib.rs`: end-to-end loader pipeline and merge/validation logic.
