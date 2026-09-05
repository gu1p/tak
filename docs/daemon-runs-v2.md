# Daemon-Owned Runs and TASKS.py v2

Tak v2 separates authoring from execution. The `tak` client loads a workspace or Makefile,
evaluates policy, resolves concrete placement candidates, and submits a complete run to the local
`takd`. The daemon owns scheduling, execution, retries, cancellation, artifacts, and persisted
events after submission.

## Start the daemon

All execution entry points require a reachable local daemon:

```bash
takd serve
tak run //:check
tak make check
tak exec -- cargo test
tak docker run alpine:3.20 echo ok
```

There is no client executor or legacy protocol fallback for `tak run`, `tak make`, `tak exec`, or
`tak docker run`. If the socket is unavailable, Tak tells you which socket it tried and to start
`takd serve`. A client timeout or disconnect does not cancel a submitted run; reconnect with the
run id instead of submitting duplicate work.

Tak, takd, and remote workers speak protocol v2 as one coordinated release. If a protocol or
`TASKS.py` version diagnostic appears, upgrade `tak` and `takd` together, upgrade workers too, and
resubmit only after checking whether the earlier run was persisted.

## Observe and recover runs

Runs outlive the terminal that submitted them:

```bash
tak runs list
tak runs show RUN_ID
tak runs attach RUN_ID
tak runs cancel RUN_ID
tak runs outputs RUN_ID --to .tmp/recovered-RUN_ID
```

- `tak runs list` shows daemon-owned runs and terminal progress.
- `tak runs show` shows run, job, node, attempt, cache, and payload-retention state.
- `tak runs attach` replays persisted events in order, follows new events, and returns the stored
  terminal exit code.
- `tak runs cancel` persists cancellation; workers may still need time to acknowledge and stop.
- `tak runs outputs ... --to DIR` creates a fresh destination and never implicitly modifies the
  submitted checkout. `DIR` must not already exist.

The submitting command attaches automatically. The first Ctrl-C requests persisted cancellation
and keeps waiting for takd to settle active work. A second Ctrl-C may detach; cancellation
continues in the daemon. Losing the terminal or network connection is only a detach: disconnect
does not cancel the run.

In a terminal, the dashboard keeps progress, nodes, tasks, the scheduler queue, and live logs
on one screen. Progress appears once at the top, including failures while other tasks continue.
Node summaries stay compact to leave room for output; focus NODES to see its active tasks and
candidate queues. This replaces the repeated per-node task lists in the default view.
Long task metadata switches to stacked rows, and log lines wrap to the terminal width.

Tab and Shift-Tab change the focused panel. Arrow keys, PgUp/PgDn, and Home/End scroll it;
the panel heading shows the visible line range when content overflows. Logs follow their tail
until scrolled back; End returns to the latest output. Short terminals show the focused panel
with tabs for the other panels. Redirected output stays plain and append-only.

After success, attach safely materializes declared outputs into the original checkout only if its
submitted paths have not changed. On a conflict, Tak copies nothing and leaves the artifacts in
takd for explicit retrieval with `tak runs outputs`.

By default, terminal logs and outputs are retained for 7 days, and terminal run metadata is retained
for 30 days. Workspace/path blobs instead use a configurable 20 GiB LRU budget; active, leased,
shared-workspace, and in-transfer data is never evicted. `tak runs show` reports when logs or outputs
have expired.

## Migrate every TASKS.py module

Every root and included module must opt into v2 explicitly:

```python
SPEC = module_spec(
    spec_version=2,
    tasks=[
        task(
            "build",
            outputs=[path("out/build.txt")],
            steps=[cmd("sh", "-c", "mkdir -p out && echo built > out/build.txt")],
            idempotent=True,
            pass_env=["BUILD_TOKEN"],
        )
    ],
)
SPEC
```

Add `spec_version=2` to included `TASKS.py` files as well as the root. `idempotent=True` allows
takd to retry an unknown or node-loss outcome with generation fencing; leave it false for work
that cannot safely run twice.

Environment inheritance is allowlisted. Use `Defaults(pass_env=[...])` or task-level `pass_env`
for stable authoring, and `--pass-env NAME` for one invocation. Requested variables must exist in
the submitting client environment. Literal step values still belong in `cmd(..., env={...})` or
`script(..., env={...})`.

## Placement selection

`RemoteSelection.Balanced()` is the default. It chooses an eligible node by projected CPU, memory,
slot, and queue pressure, with bounded cache/affinity locality credit. Use the alternatives only
when their ordering is part of the contract:

- `RemoteSelection.RoundRobin()` advances a daemon-persisted cursor, so rotation survives daemon
  restart.
- `RemoteSelection.Sequential()` always tries authored candidates from the beginning.
- `RemoteSelection.Shuffle()` was removed; migrate it to `RemoteSelection.Balanced()`.

Tak resolves Python policies and sends concrete candidates to takd. The daemon rechecks live
eligibility and capacity when it schedules each attempt.

## Sessions and affinity

Choose session reuse by the state a task actually needs:

- `SessionReuse.Workspace()` gives each job an isolated workspace snapshot. Undeclared writes are
  not shared with another job.
- `SessionReuse.Paths([...])` restores and publishes only named paths through a private CAS cache.
  Cache entries are not user outputs; declare a real `outputs=[...]` artifact for results that must
  be consumed or retrieved.
- `SessionReuse.SharedWorkspace(max_parallel_tasks=N)` shares mutable workspace state on one node
  and bounds simultaneous tasks. It requires matching hard affinity such as
  `Affinity.RequireSameNode("build-state")`.
- `SessionReuse.Container()` fuses the cascaded task group into one daemon job and container.

`Affinity.PreferSameNode("group")` is a locality hint. `Affinity.RequireSameNode("group")` is a
hard constraint and is mandatory for shared workspaces. A shared-workspace task cannot weaken its
session's hard affinity.

Example shared state:

```python
BUILD = session(
    "build-state",
    execution=Execution.Remote(),
    reuse=SessionReuse.SharedWorkspace(max_parallel_tasks=2),
    affinity=Affinity.RequireSameNode("build-state"),
)

SPEC = module_spec(
    spec_version=2,
    tasks=[task("check", use_session=BUILD, cascade_session=True)],
)
SPEC
```
