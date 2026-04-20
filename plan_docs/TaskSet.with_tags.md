# TaskSet.with_tags

## What it does

`with_tags(...)` adds tags to every task in the current set.

## Signature

```python
def with_tags(self, *tags: str) -> "TaskSet": ...
```

## Parameters

- `self`: [[TaskSet]]. Required. The set being decorated.
- `tags`: variadic `str` values. One or more tags to append.

## Returns

- [[TaskSet]]. A new set with extra tags added to every task.

## Rules

- Tags are appended to `FoundTask.template.tags`.
- Duplicate tags should be removed during implementation so the final tag list stays clean.

## Example

```python
tasks.with_tags("generated", "cargo")
```

## See also

- [[TaskSet]]
- [[TaskTemplate]]
- [[HasTag]]
