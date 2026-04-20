# FoundTask

## What it is

`FoundTask` is one discovered task before it becomes a final Tak task.

It combines a provider-local identity, a human-readable base name, a [[TaskTemplate]], and dependency information that is still partly expressed in provider terms.

## Signature

```python
@dataclass
class FoundTask:
    key: str
    name: str
    template: TaskTemplate
    deps: list[str] = field(default_factory=list)
    task_deps: list[str] = field(default_factory=list)
    metadata: dict[str, object] = field(default_factory=dict)
```

## Fields

- `key`: `str`. Required. Stable identifier inside one [[TaskSet]]. Other found tasks use this key inside `deps`.
- `name`: `str`. Required. Human-readable base name used later to build the final Tak task label.
- `template`: [[TaskTemplate]]. Required. The future task body.
- `deps`: `list[str]`. Optional. Default empty list. Internal dependencies on other found tasks in the same [[TaskSet]], expressed by `FoundTask.key`.
- `task_deps`: `list[str]`. Optional. Default empty list. External dependencies on already-known Tak task labels outside the current [[TaskSet]].
- `metadata`: `dict[str, object]`. Optional. Default empty dict. Provider-owned facts used for selection, grouping, and naming decisions.

## Rules

- `key` must be unique inside one [[TaskSet]].
- `name` must be non-empty.
- Every entry in `deps` must point to another found-task key in the same [[TaskSet]].
- Every entry in `task_deps` must already be a valid Tak label string.
- `metadata` is open-ended and owned by the provider.

## Example

```python
FoundTask(
    key="tak-core::unit",
    name="tak-core-unit",
    template=TaskTemplate(
        steps=[cmd("cargo", "test", "-p", "tak-core", "--lib")],
        tags=["cargo", "unit"],
    ),
    metadata={"package": "tak-core", "kind": "unit"},
)
```

## See also

- [[TaskTemplate]]
- [[TaskSet]]
- [[TaskKeyIn]]
- [[NameMatches]]
- [[HasTag]]
- [[MetadataEquals]]
