# Generated Tasks

## Plain-English summary

Generated tasks are not a second execution system. They are ordinary Tak tasks produced by [[TaskSet.materialize]] and merged by [[module_spec]].

After the merge, handwritten and generated tasks share the same label space, dependency rules, and execution engine.

## Why it exists

This keeps the executor simple. The discovery feature feeds the normal DAG instead of introducing a second scheduler or second runtime model.

## Related symbols

- [[TaskSet.materialize]]
- [[MaterializePlan]]
- [[module_spec]]
- [[Execution Diagram]]

## Example

If a provider emits a generated task labeled `py-tests-unit`, `tak run` treats it the same way it would treat a handwritten task with that label.
