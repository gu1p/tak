# Ergonomics and Distributed Execution Phases

This document is the single source of truth for two things:

- what Tak already ships today
- what we want to add next to make it more ergonomic and more distributed

The split matters. Tak already has useful remote and hybrid execution behavior, but some of the best UX ideas are still future work.

## Phase 1: What Tak Already Has Today

Phase 1 is shipped behavior. Everything in this section exists now.

### 1. Root-package shorthand is already ergonomic

What it is:

- At a Tak workspace root, `tak run hello` already works as shorthand for `tak run //:hello`.

What exists now:

```bash
tak run hello
tak run //:hello
```

Why it matters:

- Users do not need to type the full root label for simple root tasks.

Current boundary:

- This works at the workspace root for root-package tasks.
- Tak does not currently reinterpret bare names from arbitrary package directories such as `apps/web/` into `//apps/web:test`.

Difficulty:

- Already shipped.

### 2. Remote execution already exists

What it is:

- Tak can already execute tasks remotely.

What exists now:

- Remote placement is defined in the task DSL, usually through `RemoteOnly(...)` or a custom policy.
- Remote agents can be onboarded with `tak remote add`.
- Remote state can be inspected with `tak remote list` and `tak remote status`.

Current example:

```bash
takd init
takd serve
tak remote add "$(takd token show --wait)"
tak remote status
```

Why it matters:

- Teams can already move expensive or isolated work off the local machine.

Current boundary:

- There is no CLI override like `tak run //apps/web:test --remote`.
- Placement is task-driven today, not CLI-driven.

Difficulty:

- Already shipped.

### 3. Distributed tests are already possible, but explicit

What it is:

- Tak can already model a distributed test flow as a task graph.

What exists now:

- One local setup task.
- Multiple explicit remote test tasks.
- One final local merge or summary task.

Good shipped examples:

- [`examples/large/27_hybrid_local_remote_test_suite_success/README.md`](../examples/large/27_hybrid_local_remote_test_suite_success/README.md)
- [`examples/large/28_hybrid_local_remote_test_suite_failure_with_logs/README.md`](../examples/large/28_hybrid_local_remote_test_suite_failure_with_logs/README.md)

Why it matters:

- Users can already distribute large suites across remote work and still end with one stable summary step.

Current boundary:

- Tak does not automatically split one test target into many shards.
- Tak does not automatically merge junit, logs, or coverage for you.
- The distributed pattern is explicit today, not automatic.

Difficulty:

- Already shipped, but manual.

### 4. Remote container runtime already exists

What it is:

- Tak already supports containerized remote execution.

What exists now:

- Remote runtime is image-based through `ContainerRuntime(image=...)`.

Current example:

```python
REMOTE = Remote(
    pool="build",
    required_tags=["builder"],
    required_capabilities=["linux"],
    transport=DirectHttps(),
    runtime=ContainerRuntime(image="alpine:3.20"),
)
```

Why it matters:

- Teams can already run remote work in a controlled environment without relying only on the host machine state.

Current boundary:

- Tak does not yet accept `Dockerfile` paths as runtime inputs.
- Tak does not yet resolve repo-owned base-image definitions directly.
- Current runtime configuration is image-first.

Difficulty:

- Already shipped, but image-based only.

### 5. The current client is useful, but narrow

What it is:

- `tak web` already gives a graph UI for understanding the dependency graph.

What exists now:

- Graph visualization for targets and dependencies.

Why it matters:

- This helps users understand how tasks compose before running them.

Current boundary:

- It is not a remote-operations client.
- It does not currently act as a control plane for remote pools, logs, placement, or run aggregation.

Difficulty:

- Already shipped, but intentionally limited.

## Phase 2: Near-Term Ergonomics We Should Add

Phase 2 is the highest-value next layer. These items would make Tak much easier to use without changing its core model.

### 1. Package-relative bare task names

What we want:

- If the user is inside a package directory, allow `tak run test` to resolve to that package's task label.

Target shape:

```bash
cd apps/web
tak run test
```

Desired meaning:

```text
//apps/web:test
```

Why it matters:

- This matches how users naturally think when they are already inside a package directory.

Current gap:

- Today bare-name shorthand is effectively a root-package convenience, not a general package-relative alias system.

Difficulty:

- Medium. It changes label resolution rules and must stay predictable.

### 2. CLI remote override for runs

What we want:

- Let users ask for remote execution from the CLI when a target can be run remotely.

Target shape:

```bash
tak run //apps/web:test --remote
tak run hello --remote
```

Possible later refinements:

- `--remote=required`
- `--remote=prefer`
- `--remote-node <id>`

Why it matters:

- This is the biggest ergonomics gap today.
- It lets users experiment without rewriting `TASKS.py` first.
- It makes CI and ad hoc operator workflows simpler.

Current gap:

- Today remote choice lives in the DSL, not in the run command.

Difficulty:

- Medium to high. It needs clear precedence between task-defined placement and CLI intent.

### 3. Better visibility into why Tak chose a node

What we want:

- Make remote selection easier to understand from the CLI and the client.

Target shape:

- Clearer node-choice explanations.
- Better surfacing of pool, tag, capability, and transport filtering.
- Easier access to recent logs and run context.

Why it matters:

- Distributed systems are only ergonomic when the placement decision is explainable.

Current gap:

- Today the information exists in pieces, but the operator story is still thin.

Difficulty:

- Medium. This is mostly product and interface work, not a new execution model.

## Phase 3: Bigger Distributed Execution We Want Later

Phase 3 is the larger vision. These are the features that would make Tak feel like a stronger distributed execution system instead of only a graph executor with remote support.

### 1. Dockerfile and base-image aware remote runtimes

What we want:

- Let users point Tak at repo-owned environment definitions instead of requiring a prebuilt image reference.

Target shape:

```python
runtime=ContainerRuntime(dockerfile="images/test/Dockerfile")
```

or something conceptually similar for shared base images.

Why it matters:

- Many teams already define execution environments in `Dockerfile`s and base-image folders.
- Reusing those assets directly would make remote execution much easier to adopt.

Current gap:

- Today users must build and publish the image themselves, then reference the final image in Tak.

Difficulty:

- High. This adds build, transfer, caching, and trust questions to the runtime model.

### 2. Automatic test fan-out across multiple nodes

What we want:

- Define one logical test target and let Tak split it across multiple remote nodes, then merge the results.

Target shape:

```bash
tak run //apps/web:test --remote --shard auto
```

Why it matters:

- This is the cleanest path to large-scale distributed testing.
- Users should not have to hand-write every shard forever.

Current gap:

- Today users model shards manually as separate tasks and manually add the merge task.

Difficulty:

- High. It needs sharding strategy, scheduling, artifact aggregation, and failure reporting that still feels deterministic.

### 3. A stronger remote client

What we want:

- A client that helps operate distributed execution, not only inspect the graph.

Target shape:

- remote pool and node inventory
- live and recent runs
- placement reasons
- log access
- aggregated result views

Why it matters:

- If Tak is going to empower distributed work, users need one place to understand what happened across machines.

Current gap:

- Today `tak web` is a graph viewer, not a remote execution console.

Difficulty:

- High. This is product surface area, data plumbing, and UX work.

## Suggested Delivery Order

1. Keep documenting Phase 1 clearly so users understand what is already real.
2. Add package-relative shorthand and CLI remote overrides from Phase 2.
3. Improve node-choice and remote visibility surfaces from Phase 2.
4. Add Dockerfile or base-image aware runtime support from Phase 3.
5. Add automatic multi-node test fan-out and result merging from Phase 3.
6. Grow the client into a real distributed execution surface.

## Bottom Line

Tak already has real remote execution, hybrid local and remote pipelines, artifact roundtrip, and image-based containerized remote work.

The main opportunity is not inventing distribution from zero. The main opportunity is making the current power easier to reach, then building the bigger distributed workflow on top of that foundation.
