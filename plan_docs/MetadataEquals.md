# MetadataEquals

## What it is

`MetadataEquals` is a matcher that selects tasks by one provider-owned metadata field.

## Signature

```python
@dataclass(frozen=True)
class MetadataEquals:
    key: str
    value: object
```

## Fields

- `key`: `str`. Required. Metadata key to read from [[FoundTask]].
- `value`: `object`. Required. Exact value expected for that key.

## Rules

- Matching is exact equality.
- The metadata key must exist and equal the requested value.
- Metadata semantics are provider-owned.

## Example

```python
MetadataEquals("package", "tak-core")
```

## See also

- [[FoundTask]]
- [[TaskSet.where]]
- [[TaskSet.without]]
- [[GroupPlan]]
