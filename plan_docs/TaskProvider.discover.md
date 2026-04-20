# TaskProvider.discover

## What it does

`discover()` returns the provider's discovered tasks as one normalized [[TaskSet]].

This is the only method Tak requires from a [[TaskProvider]].

## Signature

```python
def discover(self) -> "TaskSet": ...
```

## Parameters

- `self`: [[TaskProvider]]. Required. The provider instance performing discovery.

## Returns

- [[TaskSet]]. A new task set containing the provider's discovered tasks.

## Rules

- Discovery logic lives outside Tak core.
- The returned set must use stable discovered-task keys inside its own scope.
- Providers may do richer internal work, but the result must still be a [[TaskSet]].

## Example

```python
class CargoProvider:
    def discover(self) -> TaskSet:
        return TaskSet(provider="cargo", tasks=[...])
```

## See also

- [[TaskProvider]]
- [[TaskSet]]
- [[Provider Boundary]]
