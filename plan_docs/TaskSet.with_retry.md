# TaskSet.with_retry

## What it does

`with_retry(...)` replaces the retry block on every task in the current set.

## Signature

```python
def with_retry(self, retry) -> "TaskSet": ...
```

## Parameters

- `self`: [[TaskSet]]. Required. The set being decorated.
- `retry`: existing Tak `retry(...)` object.

## Returns

- [[TaskSet]]. A new set with updated retry behavior.

## Rules

- This replaces `FoundTask.template.retry`.

## Example

```python
tasks.with_retry(retry(attempts=2, backoff=fixed(0.2)))
```

## See also

- [[TaskSet]]
- [[TaskTemplate]]
