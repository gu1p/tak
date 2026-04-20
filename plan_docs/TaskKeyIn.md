# TaskKeyIn

## What it is

`TaskKeyIn` is a matcher that selects tasks by [[FoundTask]] key.

## Signature

```python
@dataclass(frozen=True)
class TaskKeyIn:
    keys: tuple[str, ...]
```

## Fields

- `keys`: `tuple[str, ...]`. Required. One or more exact discovered-task keys to match.

## Rules

- Matching is exact.
- Order does not matter.
- Keys are compared against `FoundTask.key`, not final Tak labels.

## Example

```python
TaskKeyIn(("tak-core::unit", "tak-loader::unit"))
```

## See also

- [[FoundTask]]
- [[TaskSet.where]]
- [[TaskSet.without]]
