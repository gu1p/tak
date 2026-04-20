# Provider Boundary

## Plain-English summary

Tak owns the shared abstraction. Providers own tool-specific discovery.

Tak knows how to:

- accept a [[TaskSet]]
- shape it with methods on [[TaskSet]]
- turn it into ordinary tasks with [[TaskSet.materialize]]
- merge the result through [[module_spec]]

Providers know how to:

- inspect Cargo, uv, npm, or anything else
- collect provider-local metadata
- choose stable discovered-task keys
- expose helper methods beyond the minimal [[TaskProvider]] interface

## Why it exists

This boundary keeps Tak generic. Tak does not need to understand tool internals to schedule the result.

## Related symbols

- [[TaskProvider]]
- [[TaskProvider.discover]]
- [[TaskSet]]
- [[FoundTask]]

## Example

`CargoProvider` may parse workspace metadata and test layout. `UvProvider` may inspect Python scripts and environments. Tak only requires both providers to return a [[TaskSet]].
