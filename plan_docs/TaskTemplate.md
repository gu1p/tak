# TaskTemplate

## What it is

`TaskTemplate` is the body of a future Tak task before final naming and dependency translation happen.

It is close to `task(...)`, but it does not carry the final task label and it does not resolve discovered-task dependencies into final Tak labels.

## Signature

```python
@dataclass
class TaskTemplate:
    doc: str | None = None
    steps: list[dict] = field(default_factory=list)
    needs: list[dict] = field(default_factory=list)
    queue: dict | None = None
    retry: dict | None = None
    timeout_s: int | None = None
    context: dict | None = None
    outputs: list[dict] = field(default_factory=list)
    execution: dict | None = None
    tags: list[str] = field(default_factory=list)
```

## Fields

- `doc`: `str | None`. Optional. Default `None`. Human-facing task description.
- `steps`: `list[dict]`. Required by meaning, but defaults to an empty list. Normal Tak step definitions such as `cmd(...)` or `script(...)`.
- `needs`: `list[dict]`. Optional. Default empty list. Normal Tak `need(...)` declarations.
- `queue`: `dict | None`. Optional. Default `None`. Normal Tak queue placement object such as `queue_use(...)`.
- `retry`: `dict | None`. Optional. Default `None`. Normal Tak retry object.
- `timeout_s`: `int | None`. Optional. Default `None`. Timeout in seconds.
- `context`: `dict | None`. Optional. Default `None`. Normal Tak execution context object.
- `outputs`: `list[dict]`. Optional. Default empty list. Normal Tak output selectors such as `path(...)` or `glob(...)`.
- `execution`: `dict | None`. Optional. Default `None`. Normal Tak execution selector such as `LocalOnly(...)` or `RemoteOnly(...)`.
- `tags`: `list[str]`. Optional. Default empty list. User-facing task tags.

## Rules

- All fields keep their normal Tak meaning.
- This type does not introduce new execution semantics.
- This type describes task content, not final graph naming.

## Example

```python
TaskTemplate(
    doc="Run unit checks.",
    steps=[cmd("cargo", "test", "-p", "tak-core", "--lib")],
    tags=["unit", "cargo"],
)
```

## See also

- [[FoundTask]]
- [[TaskSet.with_execution]]
- [[TaskSet.with_retry]]
- [[TaskSet.with_timeout]]
- [[TaskSet.with_needs]]
- [[TaskSet.with_queue]]
- [[TaskSet.with_tags]]
