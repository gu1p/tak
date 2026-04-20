# TaskSet.with_queue

## What it does

`with_queue(...)` replaces queue usage on every task in the current set.

## Signature

```python
def with_queue(self, queue) -> "TaskSet": ...
```

## Parameters

- `self`: [[TaskSet]]. Required. The set being decorated.
- `queue`: existing Tak `queue_use(...)` object.

## Returns

- [[TaskSet]]. A new set with updated queue placement.

## Rules

- This replaces `FoundTask.template.queue`.

## Example

```python
tasks.with_queue(queue_use("qa_fifo", scope=MACHINE))
```

## See also

- [[TaskSet]]
- [[TaskTemplate]]
