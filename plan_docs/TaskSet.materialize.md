# TaskSet.materialize

## What it does

`materialize(...)` turns the current [[TaskSet]] into a generated task bundle that [[module_spec]] can consume.

## Signature

```python
def materialize(self, plan: MaterializePlan) -> dict: ...
```

## Parameters

- `self`: [[TaskSet]]. Required. The discovered task set being lowered.
- `plan`: [[MaterializePlan]]. Required. The naming and grouping rules for the output.

## Returns

- `dict`. A generated bundle containing ordinary Tak task definitions.

## Rules

- The result is still ordinary Tak tasks.
- No new execution unit is introduced here.
- Name collisions are errors.
- Invalid internal dependencies are errors.

## Algorithm

1. Validate that every `FoundTask.key` is unique.
2. Validate that every internal dependency in `deps` points to another task in the same set.
3. Build final Tak task names from `plan.prefix`, `plan.separator`, and `FoundTask.name`.
4. Fail if two tasks map to the same final Tak name.
5. Translate internal `deps` into final generated-task labels.
6. Keep `task_deps` as external Tak dependencies.
7. If `plan.grouping` is absent, emit generated tasks directly.
8. If grouping is present, assign one group key to every task.
9. If grouping mode is `GroupMode.SERIAL`, chain members of each group in stable final-name order.
10. If `aggregate_prefix` is set, create one aggregate task per group.
11. If `root_task` is set, create one final aggregate task.

## Example

```python
generated = tasks.materialize(
    MaterializePlan(
        prefix="cargo",
        separator="-",
        root_task="check-rust",
        grouping=GroupPlan(
            by_metadata="package",
            mode=GroupMode.PARALLEL,
            aggregate_prefix="pkg",
        ),
    )
)
```

## See also

- [[TaskSet]]
- [[MaterializePlan]]
- [[GroupPlan]]
- [[GroupMode]]
- [[Materialization]]
- [[Generated Tasks]]
- [[module_spec]]
