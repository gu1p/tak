# HasTag

## What it is

`HasTag` is a matcher that selects tasks by tag already present in [[TaskTemplate]].

## Signature

```python
@dataclass(frozen=True)
class HasTag:
    tag: str
```

## Fields

- `tag`: `str`. Required. Exact tag to look for.

## Rules

- Matching is exact.
- The comparison is done against `FoundTask.template.tags`.

## Example

```python
HasTag("integration")
```

## See also

- [[TaskTemplate]]
- [[TaskSet.where]]
- [[TaskSet.without]]
