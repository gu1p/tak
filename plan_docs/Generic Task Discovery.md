# Generic Task Discovery

## Plain-English summary

This model gives Tak one common way to work with tasks discovered by external tools without turning Tak into a Cargo-only, uv-only, or npm-only system.

The flow is simple:

1. an external provider discovers work
2. the provider returns a [[TaskSet]]
3. user code shapes that set with methods such as [[TaskSet.where]] and [[TaskSet.with_execution]]
4. [[TaskSet.materialize]] turns the result into ordinary Tak tasks
5. [[module_spec]] merges those generated tasks with handwritten ones

Tak still executes one thing only: `task`.

## Why it exists

Without a shared model, every tool integration invents its own helper layer for discovery, filtering, naming, and execution policy. This model keeps those concerns consistent while leaving tool-specific discovery outside Tak.

## Related symbols

- [[TaskProvider]]
- [[TaskSet]]
- [[FoundTask]]
- [[TaskTemplate]]
- [[MaterializePlan]]
- [[module_spec]]

## Example

A Cargo provider can discover Rust checks, a uv provider can discover Python checks, and both can return the same [[TaskSet]] shape. After that point, the same DSL applies to both.
