# Task Discovery Vault

This directory is a flat Obsidian-style vault for the generic discovery model.

Start with [[Generic Task Discovery]], then use [[Class Diagram]] or [[Execution Diagram]] if you want the shape first.

Core concepts:

- [[Generic Task Discovery]]
- [[Provider Boundary]]
- [[Materialization]]
- [[Generated Tasks]]

Types:

- [[TaskProvider]]
- [[TaskTemplate]]
- [[FoundTask]]
- [[TaskSet]]
- [[TaskKeyIn]]
- [[NameMatches]]
- [[HasTag]]
- [[MetadataEquals]]
- [[GroupMode]]
- [[GroupPlan]]
- [[MaterializePlan]]

Methods and functions:

- [[TaskProvider.discover]]
- [[TaskSet.where]]
- [[TaskSet.without]]
- [[TaskSet.with_execution]]
- [[TaskSet.with_retry]]
- [[TaskSet.with_timeout]]
- [[TaskSet.with_needs]]
- [[TaskSet.with_queue]]
- [[TaskSet.with_tags]]
- [[TaskSet.materialize]]
- [[module_spec]]

Examples:

- [[CargoProvider Example]]
- [[UvProvider Example]]
