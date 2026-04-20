# TaskSet.with_needs

## What it does

`with_needs(...)` appends needs to every task in the current set.

## Signature

```python
def with_needs(self, *needs) -> "TaskSet": ...
```

## Parameters

- `self`: [[TaskSet]]. Required. The set being decorated.
- `needs`: variadic existing Tak `need(...)` objects.

## Returns

- [[TaskSet]]. A new set with extra needs added to every task.

## Rules

- These values are appended to `FoundTask.template.needs`.
- Existing needs remain in place.

## Example

```python
tasks.with_needs(need("cpu", 2, scope=MACHINE))
```

## See also

- [[TaskSet]]
- [[TaskTemplate]]
