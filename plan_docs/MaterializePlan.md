# MaterializePlan

## What it is

`MaterializePlan` defines how a [[TaskSet]] becomes final Tak tasks.

It controls naming, optional root-task creation, and optional grouping behavior.

## Signature

```python
@dataclass(frozen=True)
class MaterializePlan:
    prefix: str
    separator: str = "-"
    root_task: str | None = None
    grouping: GroupPlan | None = None
```

## Fields

- `prefix`: `str`. Required. Prefix added to every generated task name.
- `separator`: `str`. Optional. Default `"-"`. String inserted between the prefix and the found task base name.
- `root_task`: `str | None`. Optional. Default `None`. Aggregate Tak task name that depends on all generated work in this materialization.
- `grouping`: [[GroupPlan]] `| None`. Optional. Default `None`. Grouping strategy applied before final task emission.

## Rules

- Final names must be deterministic.
- Name collisions are errors.
- Tak must not invent random fallback names or suffixes.

## Example

```python
MaterializePlan(
    prefix="cargo",
    separator="-",
    root_task="check-rust",
)
```

## See also

- [[TaskSet.materialize]]
- [[GroupPlan]]
- [[Materialization]]
