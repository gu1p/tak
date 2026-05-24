# Ergonomics and Distribution Phases

This guide keeps the current Tak authoring story separate from future distribution work. Use it as orientation before choosing examples or changing `TASKS.py`.

## Current Surface

Tak already supports project-local `TASKS.py` workspaces, explicit `module_spec(includes=[...])` composition, label-aware graph inspection, local host execution, local container execution, remote container execution, retry/timeout policy, declared outputs, and daemon-backed coordination for needs, queues, locks, resources, rate limits, and process caps.

For authoring details, prefer the source-derived bundle:

```bash
tak docs dump
```

## Next Ergonomics

- Keep README and example docs aligned with `tak docs dump`.
- Prefer source-derived command and DSL references over hand-maintained command matrices.
- Add focused docs contracts when a new command, DSL constructor, or example becomes part of the public authoring surface.
- Keep examples executable and small enough to copy into a real project.

## Distribution Direction

Tak's distribution model should continue to treat the local workspace graph as the source of truth while letting execution move between local host, local container, and configured remote agents. Remote work should stay explicit about transport, pool/tags/capabilities, container runtime, state reuse, and declared outputs so users can reason about where work ran and what came back.
