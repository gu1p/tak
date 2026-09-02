# tak CLI Architecture

## Purpose

`tak` is the short-lived authoring, resolution, and presentation client. It inspects `TASKS.py` or
Makefiles, resolves a complete v2 run, submits it to local `takd`, and renders persisted daemon
events. It does not execute task attempts and has no legacy executor fallback.

Read-only workspace commands do not require takd. Every execution entry point—`run`, `make`,
`exec`, and `docker run`—requires a reachable local daemon even for local host work.

## Runtime flow

1. Parse CLI intent and execution overrides.
2. Load and validate a v2 `TASKS.py`, adapt a Make goal, or synthesize an exec/docker task.
3. Evaluate Python policies and resolve concrete local/remote candidates.
4. Snapshot context, collect allowlisted `pass_env` values, and construct a `RunSubmission`.
5. send protocol v2 `SubmitRun` to local takd and persist the checkout association.
6. Attach to the run's ordered event stream.
7. On success, preflight and materialize declared outputs without overwriting changed checkout
   paths.

The socket comes from `TAKD_SOCKET` or the shared default runtime path. Connection failure names
that socket, tells the user to start `takd serve`, and never falls back to client execution.

## Command ownership

| Command | Client responsibility | Daemon responsibility |
|---|---|---|
| `tak list/tree/explain/graph/web` | load and present the authored graph | none |
| `tak run` | select graph, resolve policy/candidates/context/env | schedule and execute the graph |
| `tak make` | parse annotations and resolve the Make plan | schedule synthetic Make jobs |
| `tak exec` | resolve one synthetic command | execute and persist it |
| `tak docker run` | parse Docker-shaped flags and resolve a container job | execute and persist it |
| `tak runs list/show` | render persisted summaries | query the run store |
| `tak runs attach` | replay/render events and safely materialize outputs | serve ordered events/artifacts |
| `tak runs cancel` | request and report persisted cancellation | settle active work and descendants |
| `tak runs outputs --to DIR` | validate and write a fresh explicit destination | serve committed artifacts |

Remote inventory management remains a client configuration surface. At submission time tak asks
takd for protocol-v2 direct/Tor candidates; the daemon later rechecks live inventory and capacity.

## Attachment semantics

Submission and attachment are separate. A timed-out or disconnected client does not cancel the
accepted run. The run id is the recovery handle for `tak runs list`, `show`, and `attach`.

The first Ctrl-C requests persisted cancellation and continues attaching until takd acknowledges
terminal progress. A second Ctrl-C may detach. Cancellation continues in takd after detachment.

Event pages are validated for run identity, strict sequence, cursor continuity, lifecycle state,
and safe stdout/stderr encoding. Terminal task exit codes are preserved as the CLI exit status.

## Output materialization

The client records the canonical checkout root and submitted manifest per `(daemon socket, run
id)`. After a successful run it downloads outputs to a private staging directory, verifies that
submitted/output paths did not change locally, then applies the entire manifest. A conflict applies
nothing and points to `tak runs outputs RUN_ID --to DIR`.

Explicit output retrieval requires a destination that does not exist. It validates canonical
relative paths, metadata, digests, and symlink targets before creating the destination; failure
cleans the partial directory.

## Version boundary

TASKS.py v2, local daemon protocol v2, and remote worker protocol v2 ship together. Invalid or
unsupported versions produce upgrade-together guidance for tak, takd, and workers. No v1 request or
execution fallback is attempted, including when the outcome of an exchange is unknown.

## Main files

- `src/cli/command_model.rs`: public command and flag surface.
- `src/cli/run_command.rs`: v2 workspace run entrypoint.
- `src/cli/daemon_run/`: graph resolution, candidate resolution, submission, and initial attach.
- `src/cli/runs_cli.rs`: durable run recovery commands.
- `src/cli/output_materialization.rs`: checkout-safe automatic output application.
- `src/cli/run_checkout_store.rs`: private run-to-checkout association.
- `src/cli/make_cli.rs`, `exec_cli.rs`, `docker_cli.rs`: synthetic v2 submission adapters.
