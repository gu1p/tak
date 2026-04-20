# TaskSet.with_timeout

## What it does

`with_timeout(...)` replaces timeout on every task in the current set.

## Signature

```python
def with_timeout(self, timeout_s: int) -> "TaskSet": ...
```

## Parameters

- `self`: [[TaskSet]]. Required. The set being decorated.
- `timeout_s`: `int`. Required. Timeout in seconds.

## Returns

- [[TaskSet]]. A new set with updated timeout values.

## Rules

- This replaces `FoundTask.template.timeout_s`.
- `timeout_s` should be a positive integer.

## Example

```python
tasks.with_timeout(300)
```

## See also

- [[TaskSet]]
- [[TaskTemplate]]
