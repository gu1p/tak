# GroupMode

## What it is

`GroupMode` is the enum that says how tasks behave inside one group during materialization.

## Signature

```python
class GroupMode(Enum):
    PARALLEL = "parallel"
    SERIAL = "serial"
```

## Fields

- `PARALLEL`: enum value. Group members stay independent.
- `SERIAL`: enum value. Group members are chained in stable order.

## Rules

- Only these two modes exist in this design.
- Use the defined enum instead of raw strings.

## Example

```python
GroupMode.SERIAL
```

## See also

- [[GroupPlan]]
- [[TaskSet.materialize]]
- [[Materialization]]
