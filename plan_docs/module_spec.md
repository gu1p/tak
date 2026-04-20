# module_spec

## What it does

`module_spec(...)` builds the module specification for a `TASKS.py` file.

In this design, it accepts a `generated` parameter so materialized bundles can be merged with handwritten tasks before normal Tak validation.

## Signature

```python
def module_spec(
    tasks: list,
    generated: list | None = None,
    limiters: list | None = None,
    queues: list | None = None,
    exclude: list | None = None,
    includes: list | None = None,
    defaults: dict | None = None,
    project_id: str | None = None,
) -> dict: ...
```

## Parameters

- `tasks`: `list`. Required. Normal handwritten Tak tasks.
- `generated`: `list | None`. Optional. Default `None`. Materialized bundles returned by [[TaskSet.materialize]].
- `limiters`: `list | None`. Optional. Default `None`. Normal Tak limiter definitions.
- `queues`: `list | None`. Optional. Default `None`. Normal Tak queue definitions.
- `exclude`: `list | None`. Optional. Default `None`. Normal Tak exclude patterns.
- `includes`: `list | None`. Optional. Default `None`. Normal Tak include declarations.
- `defaults`: `dict | None`. Optional. Default `None`. Normal Tak defaults.
- `project_id`: `str | None`. Optional. Default `None`. Project identifier for the module.

## Returns

- `dict`. The module spec consumed by the loader.

## Rules

- `tasks` and `generated` are merged before normal Tak validation.
- Generated tasks are treated exactly like handwritten tasks after the merge.
- Duplicate labels between handwritten and generated tasks are normal Tak errors.

## Example

```python
SPEC = module_spec(
    tasks=[task("bootstrap", steps=[cmd("echo", "bootstrap")])],
    generated=[
        cargo_tasks.materialize(
            MaterializePlan(prefix="cargo", root_task="check-rust")
        ),
    ],
)
```

## See also

- [[TaskSet.materialize]]
- [[Generated Tasks]]
- [[Execution Diagram]]
