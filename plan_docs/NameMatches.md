# NameMatches

## What it is

`NameMatches` is a matcher that selects tasks by [[FoundTask]] name.

## Signature

```python
@dataclass(frozen=True)
class NameMatches:
    pattern: str
```

## Fields

- `pattern`: `str`. Required. Glob-style pattern matched against the discovered task base name.

## Rules

- Matching is done against `FoundTask.name`.
- This uses the pre-materialized name, not the final Tak label.
- It is intended for human-friendly selection.

## Example

```python
NameMatches("tak-*")
```

## See also

- [[FoundTask]]
- [[TaskSet.where]]
- [[TaskSet.without]]
