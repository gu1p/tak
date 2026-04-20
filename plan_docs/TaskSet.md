# TaskSet

## What it is

`TaskSet` is the shared container Tak uses for discovered tasks.

Every provider returns this shape. After that point, users work with the same DSL whether the tasks came from Cargo, uv, or another tool.

## Signature

```python
@dataclass
class TaskSet:
    provider: str
    tasks: list[FoundTask]
    doc: str | None = None
```

## Fields

- `provider`: `str`. Required. Provider name such as `"cargo"` or `"uv"`.
- `tasks`: `list[FoundTask]`. Required. The discovered tasks in this set.
- `doc`: `str | None`. Optional. Default `None`. Human-facing description of the whole set.

## Rules

- This is the shared container type inside Tak.
- Providers return this type through [[TaskProvider.discover]].
- DSL methods on this type return new shaped [[TaskSet]] values instead of mutating the original one.
- Discovery details stay outside Tak. Tak only standardizes the result shape.

## Example

```python
tasks = cargo.discover()
docker_subset = tasks.where(MetadataEquals("kind", "integration"))
```

## See also

- [[TaskProvider]]
- [[FoundTask]]
- [[TaskSet.where]]
- [[TaskSet.without]]
- [[TaskSet.with_execution]]
- [[TaskSet.with_retry]]
- [[TaskSet.with_timeout]]
- [[TaskSet.with_needs]]
- [[TaskSet.with_queue]]
- [[TaskSet.with_tags]]
- [[TaskSet.materialize]]
