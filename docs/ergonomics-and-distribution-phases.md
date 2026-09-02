# Ergonomics and Distribution Phases

This historical guide separates authoring concerns from distribution design. For the shipped v2
contract, use [Daemon-Owned Runs and TASKS.py v2](daemon-runs-v2.md).

## Current Surface

Tak supports explicit `module_spec(spec_version=2, includes=[...])` workspaces, label-aware graph
inspection, local/remote runtimes, declared outputs, and daemon-owned scheduling, retries,
cancellation, artifacts, and persisted run events.

For authoring details, prefer the source-derived bundle:

```bash
tak docs dump
```

## Next Ergonomics

- Keep README and example docs aligned with `tak docs dump`.
- Prefer source-derived command and DSL references over hand-maintained command matrices.
- Add focused docs contracts when a new command, DSL constructor, or example becomes part of the public authoring surface.
- Keep examples executable and small enough to copy into a real project.

## Distribution Contract

The local workspace graph remains the source of authoring truth. Tak resolves policies and concrete
candidates; local takd owns placement and execution across local host, local container, and
configured direct/Tor workers. Transport, requirements, runtime, reuse, environment, and outputs
stay explicit so a persisted run is reproducible and inspectable.
