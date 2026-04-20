# GroupPlan

## What it is

`GroupPlan` describes how found tasks should be grouped before final materialization.

## Signature

```python
@dataclass(frozen=True)
class GroupPlan:
    by_metadata: str | None = None
    by_name_separator: str | None = None
    by_name_depth: int = 1
    mode: GroupMode = GroupMode.PARALLEL
    aggregate_prefix: str | None = None
```

## Fields

- `by_metadata`: `str | None`. Optional. Default `None`. Metadata key whose value becomes the group key.
- `by_name_separator`: `str | None`. Optional. Default `None`. Separator used when grouping by name parts.
- `by_name_depth`: `int`. Optional. Default `1`. Number of split name segments to use when building the group key from `FoundTask.name`.
- `mode`: [[GroupMode]]. Optional. Default `GroupMode.PARALLEL`. Controls whether tasks inside a group stay parallel or become serial.
- `aggregate_prefix`: `str | None`. Optional. Default `None`. Prefix used to create one aggregate task per group.

## Rules

- Exactly one grouping source should be active: `by_metadata` or `by_name_separator`.
- If neither grouping source is set, there is no grouping.
- If a task does not produce a group key, it becomes a group of one.
- `by_name_depth` must be at least `1`.

## Example

```python
GroupPlan(
    by_metadata="package",
    mode=GroupMode.PARALLEL,
    aggregate_prefix="pkg",
)
```

## See also

- [[GroupMode]]
- [[MaterializePlan]]
- [[TaskSet.materialize]]
