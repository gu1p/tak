# TaskSet.where

## What it does

`where(...)` keeps only tasks that match at least one matcher.

## Signature

```python
def where(self, *matchers) -> "TaskSet": ...
```

## Parameters

- `self`: [[TaskSet]]. Required. The source set being filtered.
- `matchers`: variadic matcher values. Supported examples include [[TaskKeyIn]], [[NameMatches]], [[HasTag]], and [[MetadataEquals]].

## Returns

- [[TaskSet]]. A new set containing only matching tasks.

## Rules

- If at least one matcher is given, a task stays only if it matches one or more matchers.
- This method does not silently re-add dependencies.
- Use this before [[TaskSet.materialize]] when you want a subset.

## Example

```python
cargo.discover().where(MetadataEquals("kind", "integration"))
```

## See also

- [[TaskSet]]
- [[TaskSet.without]]
- [[TaskKeyIn]]
- [[NameMatches]]
- [[HasTag]]
- [[MetadataEquals]]
