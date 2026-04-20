# TaskSet.without

## What it does

`without(...)` removes tasks that match at least one matcher.

## Signature

```python
def without(self, *matchers) -> "TaskSet": ...
```

## Parameters

- `self`: [[TaskSet]]. Required. The source set being filtered.
- `matchers`: variadic matcher values. Supported examples include [[TaskKeyIn]], [[NameMatches]], [[HasTag]], and [[MetadataEquals]].

## Returns

- [[TaskSet]]. A new set with matching tasks removed.

## Rules

- If a task matches any matcher, it is removed.
- Use this when you want to keep most of the set and cut out a smaller subset.

## Example

```python
cargo.discover().without(HasTag("slow"))
```

## See also

- [[TaskSet]]
- [[TaskSet.where]]
- [[TaskKeyIn]]
- [[NameMatches]]
- [[HasTag]]
- [[MetadataEquals]]
