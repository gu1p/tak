# TaskProvider

## What it is

`TaskProvider` is the minimal interface for anything that can discover tasks for Tak.

Tak does not care how the provider gathers data. It only cares that the provider can return a [[TaskSet]] through [[TaskProvider.discover]].

## Signature

```python
class TaskProvider(Protocol):
    def discover(self) -> "TaskSet": ...
```

## Fields

- None. This is an interface, not a concrete data container.

## Rules

- Implemented outside Tak core.
- Must provide [[TaskProvider.discover]].
- May expose richer helper methods outside this minimal interface.
- Tak depends only on the returned [[TaskSet]] shape.

## Example

```python
class CargoProvider:
    def discover(self) -> TaskSet:
        ...
```

## See also

- [[TaskProvider.discover]]
- [[TaskSet]]
- [[Provider Boundary]]
