# TaskSet.with_execution

## What it does

`with_execution(...)` replaces the execution block on every task in the current set.

## Signature

```python
def with_execution(self, execution) -> "TaskSet": ...
```

## Parameters

- `self`: [[TaskSet]]. Required. The set being decorated.
- `execution`: existing Tak execution object. Examples include `Execution.Local(...)`, `Execution.Remote(...)`, or another valid execution selector.

## Returns

- [[TaskSet]]. A new set with updated execution on every task.

## Rules

- This replaces `FoundTask.template.execution`.
- It does not change steps, dependencies, or outputs.

## Example

```python
integration_tasks.with_execution(
    Execution.Local(runtime=TEST_DOCKER)
)
```

## See also

- [[TaskSet]]
- [[TaskTemplate]]
- [[TaskSet.materialize]]
