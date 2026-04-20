# UvProvider Example

## Plain-English summary

This example shows how a uv-specific provider can discover Python checks, narrow the set to test tasks, remove slow tasks, and materialize the remainder as ordinary Tak tasks.

## Why it exists

This proves the model is not Rust-specific. The same [[TaskSet]] DSL can shape Python-discovered work too.

## Related symbols

- [[TaskProvider]]
- [[TaskProvider.discover]]
- [[TaskSet.where]]
- [[TaskSet.without]]
- [[TaskSet.with_tags]]
- [[TaskSet.with_timeout]]
- [[TaskSet.materialize]]
- [[module_spec]]

## Example

Provider sketch:

```python
class UvProvider:
    def __init__(self, project_root: str):
        self.project_root = project_root

    def discover(self) -> TaskSet:
        return TaskSet(
            provider="uv",
            doc="Python tasks discovered from the uv project.",
            tasks=[
                FoundTask(
                    key="lint",
                    name="lint",
                    template=TaskTemplate(
                        steps=[cmd("uv", "run", "ruff", "check", ".")],
                        tags=["python", "lint"],
                    ),
                    metadata={"kind": "check", "tool": "ruff"},
                ),
                FoundTask(
                    key="tests-unit",
                    name="tests-unit",
                    template=TaskTemplate(
                        steps=[cmd("uv", "run", "pytest", "tests/unit")],
                        tags=["python", "test"],
                    ),
                    metadata={"kind": "check", "suite": "unit"},
                ),
                FoundTask(
                    key="tests-e2e",
                    name="tests-e2e",
                    template=TaskTemplate(
                        steps=[cmd("uv", "run", "pytest", "tests/e2e")],
                        tags=["python", "test", "slow"],
                    ),
                    metadata={"kind": "check", "suite": "e2e"},
                ),
            ],
        )
```

Shaping chain:

```python
uv = UvProvider(project_root=".")

python_checks = (
    uv.discover()
    .where(MetadataEquals("kind", "check"))
    .where(NameMatches("tests-*"))
    .without(HasTag("slow"))
    .with_tags("generated", "uv")
    .with_timeout(300)
)
```

Materialization:

```python
SPEC = module_spec(
    tasks=[],
    generated=[
        python_checks.materialize(
            MaterializePlan(
                prefix="py",
                separator="-",
                root_task="check-python",
            )
        ),
    ],
)
SPEC
```

Expected generated tasks:

- `py-tests-unit`
- `check-python`
