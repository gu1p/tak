# Ergonomics and Distributed Execution

This document describes two things:

- the Tak model that exists today
- the replacement ergonomics model we want to move toward

The point of the replacement is simple: most users should not need to think about transport,
tags, capabilities, or placement constructors just to create `TASKS.py` and run work.

Tak should be opinionated by default, easy for newcomers, and still fully configurable for power
users.

## Today

Tak already ships useful behavior.

### Root shorthand already exists

At a workspace root:

```bash
tak run hello
tak run //:hello
```

Those already mean the same thing.

Current boundary:

- this only works for root-package tasks
- Tak does not currently reinterpret bare names from arbitrary package directories into
  package-relative labels

### Remote execution already exists

Tak can already run tasks remotely.

Today that is mostly task-driven:

- `RemoteOnly(...)`
- `ByCustomPolicy(...)`
- explicit `Remote(...)` requirements in `TASKS.py`

Remote agents can already be onboarded and inspected:

```bash
takd init
takd serve
tak remote add "$(takd token show --wait)"
tak remote status
```

Current boundary:

- there is no `tak run ... --remote` or `tak run ... --local`
- placement is mostly encoded in the task definition itself

### Distributed flows already exist, but they are explicit

Tak can already model hybrid local and remote pipelines and explicit distributed test graphs.

Good shipped examples:

- [`examples/large/27_hybrid_local_remote_test_suite_success/README.md`](../examples/large/27_hybrid_local_remote_test_suite_success/README.md)
- [`examples/large/28_hybrid_local_remote_test_suite_failure_with_logs/README.md`](../examples/large/28_hybrid_local_remote_test_suite_failure_with_logs/README.md)

Current boundary:

- users write explicit remote tasks themselves
- users manually define the local setup and local merge steps
- there is no automatic sharding or result aggregation

### Remote runtime configuration already exists

Tak already supports remote container runtime configuration:

```python
REMOTE = Remote(
    pool="build",
    required_tags=["builder"],
    required_capabilities=["linux"],
    transport=DirectHttps(),
    runtime=ContainerRuntime(image="alpine:3.20"),
)
```

Current boundary:

- this is still explicit-first
- even common remote workflows currently ask the user to think about advanced filters early

## Replacement Model

The replacement model is not "more knobs." It is "better defaults."

The common case should be:

- write a task
- run the task
- let Tak decide where it should run using opinionated defaults

The uncommon case should still exist:

- explicit transport filters
- explicit tags or capabilities
- explicit hard placement constructors
- custom scheduling policies

Those stay available, but they are advanced controls, not the default starting point.

### Default behavior

For most tasks, the user should be able to omit execution details entirely.

Conceptually:

```python
SPEC = module_spec(
    tasks=[
        task(
            "test",
            steps=[cmd("pytest", "-q")],
        ),
    ]
)
SPEC
```

Then:

```bash
tak run test
```

Tak should use a default placement policy instead of forcing the author to first decide between
`LocalOnly(...)`, `RemoteOnly(...)`, transport, tags, and capability filters.

### Default policy: remote first

The default newcomer policy should be `remote_first`.

Meaning:

1. find eligible remote nodes
2. randomize them fairly using one seeded shuffle per task run
3. submit to them in sequence
4. stop at the first node that accepts the work
5. if all eligible remotes reject or are unavailable, run locally last if local execution is allowed

This keeps Tak simple to use while still favoring remote capacity when it exists.

### Node admission uses EAFP

Tak should not try to perfectly predict whether a node can run the task.

Instead, the node should be the source of truth for admission.

The client behavior should be EAFP:

1. submit to a candidate node
2. if the node accepts, the placement is done
3. if the node rejects for capacity-ish reasons, move to the next candidate
4. if no remote candidate accepts, fall back to local when policy allows

This avoids trying to make the client smarter than the nodes.

### Which failures move to the next candidate

Retry-next-candidate should be limited to the cases that actually mean "try somewhere else":

- node unavailable
- node busy
- node policy reject
- not enough current margin or resources on that node

Tak should fail fast instead of silently falling through when the error is not about capacity:

- auth failures
- protocol mismatches
- malformed responses
- broken configuration

Those are operator errors, not scheduling signals.

### Transport is usually not a filter

Most users do not care whether a remote uses direct transport or Tor transport.

In the replacement model:

- omitted transport means `any`
- transport should only be specified when the user explicitly needs to constrain it

That makes transport an advanced filter instead of part of the normal path.

### Pool, tags, and capabilities are also advanced filters

Most users should not start from:

- `pool=...`
- `required_tags=...`
- `required_capabilities=...`

In the replacement model:

- omitted pool means no pool filter
- omitted tags mean no tag filter
- omitted capabilities mean no capability filter

Tak should first try to make the simple path work. Filters remain available when the user needs to
target a specialized class of nodes.

## User Config

The replacement model needs one concrete place for defaults.

The first version should use one user config file:

```text
$HOME/.tak/tak.toml
```

This file should be auto-created with comments the first time Tak needs it.

It should hold both:

- remote inventory
- default placement policy

### Proposed file shape

```toml
version = 1

# Built-in placement policies:
# - remote_first
# - local_first
# - force_remote
# - force_local
#
# Soft CLI flags:
# - tak run ... --remote
# - tak run ... --local
#
# Hard CLI overrides:
# - tak run ... --force-remote
# - tak run ... --force-local
default_policy = "remote_first"

[remote_defaults]
# Most users should not care about transport.
# "any" means Tak accepts direct or tor remotes.
transport = "any"

# Empty means "do not filter".
pool = ""
tags = []
capabilities = []

[[remotes]]
node_id = "builder-a"
display_name = "builder-a"
base_url = "http://builder-a"
bearer_token = "secret"
pools = ["build"]
tags = ["builder"]
capabilities = ["linux"]
transport = "direct"
enabled = true

[[remotes]]
node_id = "builder-b"
display_name = "builder-b"
base_url = "http://builder-b.onion"
bearer_token = "secret"
pools = ["build"]
tags = ["builder"]
capabilities = ["linux"]
transport = "tor"
enabled = true
```

### Why one file

One file keeps the model easy to explain:

- where Tak finds remotes
- which policy Tak uses by default
- which advanced defaults exist when the user wants them

The goal is low cognitive load, not perfect separation.

## CLI Semantics

The replacement model should add four placement flags:

```bash
tak run //apps/web:test --remote
tak run //apps/web:test --local
tak run //apps/web:test --force-remote
tak run //apps/web:test --force-local
```

### Soft preference flags

`--remote` and `--local` are soft preferences.

They should bias placement without pretending to be stronger than every task rule.

Examples:

- `--remote` means "prefer remote for this run"
- `--local` means "prefer local for this run"

### Hard override flags

`--force-remote` and `--force-local` are hard overrides.

They exist for the cases where the operator explicitly wants to cross the normal placement rule.

Examples:

- forcing a remote run for validation
- forcing a local run while debugging a remote problem

### Precedence

The intended precedence is:

1. `--force-remote` / `--force-local`
2. explicit task hard constraints
3. soft flags `--remote` / `--local`
4. `default_policy` from `$HOME/.tak/tak.toml`
5. inherited remote defaults when execution details are omitted

## Hard Constraints Stay Available

The replacement model does not remove advanced controls.

These still matter:

- `LocalOnly(...)`
- `RemoteOnly(...)`
- `ByCustomPolicy(...)`
- explicit `Remote(...)` filters

But they should be treated as advanced controls.

The normal path should be "write the task and let the defaults work."

## Placement Visibility

If Tak falls through multiple candidates, the CLI should say so.

The operator should be able to see:

- which policy was used
- which remotes were considered
- the shuffled candidate order for that task run
- which node accepted the task
- why previous candidates were skipped or rejected
- when local fallback happened and why

Distributed systems only feel ergonomic when the decision is understandable.

## Newcomer Path

The newcomer path should look like this:

### 1. Write a simple task

```python
SPEC = module_spec(
    tasks=[
        task(
            "test",
            steps=[cmd("pytest", "-q")],
        ),
    ]
)
SPEC
```

### 2. Run it

```bash
tak run test
```

### 3. Only reach for advanced filters when needed

Examples of advanced situations:

- "this must run on a linux builder"
- "this must run only in a specific pool"
- "this must use a specific transport"
- "this must never run locally"

Those cases are real. They are just not the common starting point.

## What This Replaces

This model replaces the need to treat these concepts as part of the default authoring path:

- explicit placement constructors for common tasks
- explicit transport selection for common remote usage
- explicit tags and capabilities for common remote usage

Tak should still support them. It just should not make newcomers carry them mentally from the
start.

## Bottom Line

Tak already has real remote execution.

The replacement ergonomics model is about making that power easy to reach:

- one user config file
- one default policy
- remote-first by default
- seeded fair remote ordering
- EAFP node admission
- local last fallback
- advanced filters only when they are actually needed

That is the path to making `TASKS.py` easy for newcomers without taking control away from power
users.
