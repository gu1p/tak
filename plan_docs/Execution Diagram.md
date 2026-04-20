# Execution Diagram

```mermaid
flowchart TD
    A["Tak user code"] --> B["Provider implementation<br/>outside Tak"]
    B --> C["discover() returns<br/>TaskSet"]
    C --> D["Shape the set with<br/>where() or without()"]
    D --> E["Decorate the set with<br/>execution, retry, timeout,<br/>needs, queue, and tags"]
    E --> F["materialize() applies<br/>MaterializePlan"]
    F --> G["Generated Tak tasks"]
    G --> H["module_spec() merges<br/>handwritten and generated tasks"]
    H --> I["Loader validates labels,<br/>dependencies, and cycles"]
    I --> J["tak list / tak explain / tak graph"]
    I --> K["tak run executes<br/>the normal Tak DAG"]
```

## Explanation

- Discovery happens outside Tak through a provider that satisfies [[TaskProvider]].
- The provider returns a [[TaskSet]] through [[TaskProvider.discover]].
- Selection and decoration happen on that set through methods on [[TaskSet]].
- [[TaskSet.materialize]] lowers the discovered representation into [[Generated Tasks]].
- [[module_spec]] merges generated tasks with handwritten ones before normal validation.
- From that point on, Tak uses its normal graph and execution behavior.

## Related symbols

- [[TaskProvider]]
- [[TaskSet]]
- [[TaskSet.materialize]]
- [[MaterializePlan]]
- [[Generated Tasks]]
- [[module_spec]]
