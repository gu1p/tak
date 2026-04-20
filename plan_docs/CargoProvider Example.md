# CargoProvider Example

## Plain-English summary

This example shows how a Cargo-specific provider can discover Rust tasks, shape a subset, move that subset into container-backed execution, and materialize the result as ordinary Tak tasks.

## Why it exists

Cargo is a good example of the provider boundary. Discovery is tool-specific, but the returned shape is still the generic [[TaskSet]].

## Related symbols

- [[TaskProvider]]
- [[TaskProvider.discover]]
- [[TaskSet.where]]
- [[TaskSet.with_execution]]
- [[TaskSet.with_timeout]]
- [[TaskSet.with_tags]]
- [[TaskSet.materialize]]
- [[module_spec]]

## Example

Provider sketch:

```python
class CargoProvider:
    def __init__(self, workspace_root: str):
        self.workspace_root = workspace_root

    def discover(self) -> TaskSet:
        return TaskSet(
            provider="cargo",
            doc="Rust tasks discovered from the Cargo workspace.",
            tasks=[
                FoundTask(
                    key="tak-core::unit",
                    name="tak-core-unit",
                    template=TaskTemplate(
                        steps=[cmd("cargo", "test", "-p", "tak-core", "--lib")],
                        tags=["cargo", "unit"],
                    ),
                    metadata={"package": "tak-core", "kind": "unit"},
                ),
                FoundTask(
                    key="tak::integration",
                    name="tak-integration",
                    template=TaskTemplate(
                        steps=[cmd("cargo", "test", "-p", "tak", "--lib")],
                        tags=["cargo", "integration"],
                    ),
                    metadata={"package": "tak", "kind": "integration"},
                ),
            ],
        )
```

Shaping chain:

```python
TEST_DOCKER = DockerfileRuntime(dockerfile=path("docker/tak-tests/Dockerfile"))

cargo = CargoProvider(workspace_root=".")

cargo_integration = (
    cargo.discover()
    .where(MetadataEquals("kind", "integration"))
    .with_execution(LocalOnly(Local(id="docker-local", runtime=TEST_DOCKER)))
    .with_timeout(600)
    .with_tags("generated", "dockerized")
)
```

Materialization:

```python
SPEC = module_spec(
    tasks=[
        task("bootstrap", steps=[cmd("echo", "bootstrap")]),
    ],
    generated=[
        cargo_integration.materialize(
            MaterializePlan(
                prefix="cargo",
                separator="-",
                root_task="check-rust",
            )
        ),
    ],
)
SPEC
```

Expected generated tasks:

- `cargo-tak-integration`
- `check-rust`
