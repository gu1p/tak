# Materialization

## Plain-English summary

Materialization is the boundary where discovered tasks become ordinary Tak tasks.

Before materialization, tasks are described with provider-local keys, provider metadata, and pre-final names. After materialization, tasks have final Tak labels and normal Tak dependencies.

## Why it exists

Discovery and shaping need freedom. Execution needs stable Tak tasks. [[TaskSet.materialize]] is the step that lowers one world into the other.

## Related symbols

- [[TaskSet.materialize]]
- [[MaterializePlan]]
- [[GroupPlan]]
- [[GroupMode]]
- [[Generated Tasks]]

## Example

A found task with key `tak-core::unit` and name `tak-core-unit` can become a final task like `cargo-tak-core-unit` after [[TaskSet.materialize]] applies a [[MaterializePlan]] with prefix `cargo`.
