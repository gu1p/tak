# User-First Remote Orchestration Roadmap

This roadmap is ordered by what the user notices first:

1. Can I see what each node can do right now?
2. Can I see what each node is already doing?
3. Can I configure a node without hand-editing config?
4. Does a busy node protect itself clearly?
5. Does the client choose a better node automatically?
6. Can I express remote intent in `TASKS.py` with much less ceremony?

One checkbox should correspond to one small, reviewable PR.

## Operating Rules

- Use strict Red -> Green -> Refactor for every feature task.
- Start with BDD / UI contract tests for every user-visible behavior.
- Add unit tests for model, policy, and ranking changes before implementation.
- Add integration tests for CLI, protocol, and config boundaries before implementation.
- Run `make check` for code changes.
- Do not run `make check` for doc-only roadmap edits.
- Do not batch two user-visible behaviors into one checkbox.
- Do not silently change CLI wording, columns, layout, or setup flow without updating tests first.

## Completion Evidence (Required In Each PR)

- Tests added:
- Command run:
- Result: pass/fail
- User-visible command(s) covered:
- Follow-up tasks unlocked:

## Product Decisions Locked For This Roadmap

- `tak remote list` is the operator entrypoint.
- In a TTY, `tak remote list` opens a `ratatui` fleet dashboard by default.
- `tak remote list --plain` remains the stable script-friendly output.
- `tak remote status --node <id>` remains the plain per-node detail command.
- The `ratatui` fleet dashboard is a beautiful operator UI, not a debug dump.
- The fleet dashboard shows live node signals in V1: CPU, RAM, storage, running, queued, admission.
- The fleet dashboard also shows selected-node queue details in V1.
- Resource visibility in V1 means runtimes plus declared capacity summaries.
- Queue visibility in V1 means queue name, policy type, max running, max queued, running count, queued count, and acceptance reason.
- First shipped queue policy types are `reject-fast`, `bounded-fifo`, and `drain`.
- First shipped queue discipline is FIFO only.
- Client ranking uses hard filters first, then admission/queue health, then utilization signals.

## Milestone 1: `tak remote list` Becomes The Fleet Dashboard

### User Outcome

When a user runs `tak remote list` in a terminal, they get a `ratatui` dashboard that answers:

- what nodes exist
- what each node can do
- how busy each node is
- whether each node is accepting, draining, or unreachable
- what queues exist on the selected node

When they need stable scripting output, they run `tak remote list --plain`.

### Atomic Tasks

- [ ] `BDD/UI` `tak remote list` opens a full-screen `ratatui` dashboard when stdout is a TTY.
- [ ] `BDD` `tak remote list --plain` prints a stable non-TUI fleet table.
- [ ] `BDD/UI` the dashboard table includes visible columns for node, transport, runtimes, CPU, RAM, storage, running, queued, and admission.
- [ ] `BDD/UI` the dashboard includes a selected-node detail pane for queues.
- [ ] `BDD/UI` the queue pane shows queue name, policy, running/max, queued/max, and acceptance reason.
- [ ] `BDD/UI` the dashboard shows a loading state before the first live refresh completes.
- [ ] `BDD/UI` the dashboard keeps unreachable nodes visible with a clear offline or stale state.
- [ ] `BDD/UI` moving selection up or down changes the queue pane without changing the fleet table shape.
- [ ] `BDD/UI` the dashboard footer shows refresh status and last error without obscuring the table.
- [ ] `BDD/UI` the dashboard exits cleanly with `q` and `Esc`.
- [ ] `Unit` add `ResourceProfile` with CPU, memory, and storage capacity fields.
- [ ] `Unit` add `QueueDefinition` with queue name, policy, max running, and max queued.
- [ ] `Unit` add `QueueSnapshot` with running count, queued count, accepting state, and reason.
- [ ] `Unit` add `AdmissionState` for node-wide accepting, draining, full, and offline states.
- [ ] `Unit` add serialization coverage for inventory persistence of resource and queue metadata.
- [ ] `Integration` extend node info to return static resource profile and queue definitions.
- [ ] `Integration` extend node status to return live queue snapshots and admission state.
- [ ] `Integration` `tak remote add` captures and persists the node's static resource and queue metadata after a successful probe.
- [ ] `Implementation` add `--plain` to `tak remote list`.
- [ ] `Implementation` add the `ratatui` dashboard shell for `tak remote list`.
- [ ] `Implementation` add the fleet table renderer with fixed column widths that do not jump.
- [ ] `Implementation` add the selected-node queue pane renderer.
- [ ] `Implementation` add a periodic live refresh loop with stable row ordering.
- [ ] `Implementation` add stale-state and error-state footer messaging.
- [ ] `Docs` update the first `tak remote list` example to show TTY dashboard behavior and `--plain`.

## Milestone 2: Queue Inspection Becomes A First-Class Workflow

### User Outcome

After seeing that a node is busy, the user can inspect that node and understand:

- which queues exist
- what policy each queue uses
- how much room remains
- whether a task is likely to start now, wait, or be rejected

### Atomic Tasks

- [ ] `BDD` `tak remote status --node <id>` prints one row per queue.
- [ ] `BDD` `tak remote status --node <id>` shows queue name, policy, running/max, queued/max, and acceptance reason.
- [ ] `BDD` `tak remote status --node <id>` repeats the node admission state in the header.
- [ ] `BDD` `tak remote status --node <id>` shows an explicit empty-state message when the node has no named queues.
- [ ] `BDD` `tak remote status --watch --node <id>` updates queue counters without losing row order.
- [ ] `BDD` `tak remote status --watch --node <id>` keeps an offline or error state visible on transient failures.
- [ ] `Unit` add formatting helpers for queue utilization strings.
- [ ] `Unit` add formatting helpers for queue rejection and draining reasons.
- [ ] `Unit` add mapping coverage from protocol queue state to CLI display state.
- [ ] `Integration` queue detail responses distinguish `queue full` from `node draining`.
- [ ] `Integration` queue lists are returned in stable order so watch mode does not jump.
- [ ] `Implementation` add queue utilization formatting to plain status output.
- [ ] `Implementation` add queue reason formatting to plain status output.
- [ ] `Implementation` keep watch-mode row ordering stable across refreshes.
- [ ] `Docs` add a troubleshooting example that starts with `tak remote list` and drills into `tak remote status --node`.

## Milestone 3: `takd` Setup Becomes Guided Instead Of Manual

### User Outcome

A user should be able to bootstrap a node with:

- `curl ... | sh -- --interactive`

and get a guided `ratatui` setup flow that configures:

- node identity
- transport
- pools, tags, and runtimes
- CPU, RAM, and storage capacity
- queue names and sizes
- queue policy types

without hand-editing config files.

The existing non-interactive path stays available.

### Atomic Tasks

- [ ] `BDD` the install script accepts `--interactive` and forwards it to `takd init`.
- [ ] `BDD` the install script keeps the existing non-interactive behavior unchanged when `--interactive` is absent.
- [ ] `BDD/UI` the setup opens on a node identity step with visible fields for node id and display name.
- [ ] `BDD/UI` the setup has a transport step with visible supported-transport choices.
- [ ] `BDD/UI` the setup has a capabilities step for pools, tags, and runtimes.
- [ ] `BDD/UI` the setup has a resources step for CPU, RAM, and storage capacity fields.
- [ ] `BDD/UI` the setup has a queues step for queue name, max running, and max queued.
- [ ] `BDD/UI` the setup has a policy step with visible choices for `reject-fast`, `bounded-fifo`, and `drain`.
- [ ] `BDD/UI` the setup has a review step that shows the full configuration before save.
- [ ] `BDD/UI` cancel exits cleanly without partial config writes.
- [ ] `BDD/UI` invalid numeric values block save with visible validation.
- [ ] `Unit` add CLI parsing coverage for `takd init --interactive`.
- [ ] `Unit` extend persisted node config with resource and queue-policy fields.
- [ ] `Unit` add validation for queue names, queue sizes, and policy-specific limits.
- [ ] `Integration` interactive save produces the same config shape as the non-interactive path.
- [ ] `Integration` rerunning interactive setup preloads current values.
- [ ] `Implementation` add install-script forwarding for `--interactive`.
- [ ] `Implementation` add `takd init --interactive`.
- [ ] `Implementation` add the `ratatui` setup shell.
- [ ] `Implementation` add the identity step.
- [ ] `Implementation` add the transport step.
- [ ] `Implementation` add the capabilities step.
- [ ] `Implementation` add the resources step.
- [ ] `Implementation` add the queues step.
- [ ] `Implementation` add the policy step.
- [ ] `Implementation` add the review-and-save step.
- [ ] `Docs` update bootstrap docs to show both the fast non-interactive path and the guided interactive path.

## Milestone 4: Busy Nodes Protect Themselves Explicitly

### User Outcome

When a node is under load, the user sees one of three clear outcomes:

- the task starts now
- the task is accepted into a queue
- the task is rejected with a clear reason

The node stops pretending to accept work it cannot actually run.

### Atomic Tasks

- [ ] `BDD` submitting to an idle node returns an explicit `started` outcome.
- [ ] `BDD` submitting to a saturated queueing node returns an explicit `queued` outcome.
- [ ] `BDD` submitting to a full `reject-fast` queue returns an explicit `rejected` outcome with a capacity reason.
- [ ] `BDD` submitting to a draining node returns an explicit `rejected` outcome with a draining reason.
- [ ] `BDD` queued tasks appear in the dashboard and status output before execution starts.
- [ ] `BDD` canceling queued tasks removes them from the queue before execution starts.
- [ ] `BDD` queued tasks advance to running when capacity frees up.
- [ ] `Unit` add an admission-policy evaluator for `reject-fast`.
- [ ] `Unit` add an admission-policy evaluator for `bounded-fifo`.
- [ ] `Unit` add an admission-policy evaluator for `drain`.
- [ ] `Unit` add FIFO queue-position accounting coverage.
- [ ] `Unit` add running and queued slot accounting coverage.
- [ ] `Unit` add persistence coverage for queued jobs across restart.
- [ ] `Integration` submission uses admission control before spawning runtime work.
- [ ] `Integration` queue promotion occurs when a running job completes.
- [ ] `Integration` queued jobs reload after node restart.
- [ ] `Integration` event streams distinguish `queued` from `started`.
- [ ] `Implementation` add the node admission layer ahead of runtime spawn.
- [ ] `Implementation` add queue storage for accepted-but-not-running jobs.
- [ ] `Implementation` add queue promotion on slot release.
- [ ] `Implementation` add cancel handling for queued jobs.
- [ ] `Implementation` add queue and admission counters to node status reporting.
- [ ] `Docs` document `started`, `queued`, and `rejected` in user terms.

## Milestone 5: `tak run` Chooses Better Nodes Before It Submits

### User Outcome

A user can run `tak run <task>` and trust that the client will:

- skip nodes that cannot run the task
- prefer nodes that are accepting work
- avoid clearly overloaded nodes
- explain why a node was selected

### Atomic Tasks

- [ ] `BDD` client-side selection excludes nodes that fail hard runtime or resource requirements.
- [ ] `BDD` client-side selection prefers accepting nodes over draining nodes.
- [ ] `BDD` client-side selection prefers queue room over immediate rejection.
- [ ] `BDD` when multiple nodes can queue the task, the client prefers the healthier and shorter queue.
- [ ] `BDD` when the selected node rejects for capacity, the client retries the next eligible node before failing.
- [ ] `BDD` run output includes a plain-language placement reason for the selected node.
- [ ] `BDD` when all eligible nodes are full or draining, the run output explains failure by node and reason.
- [ ] `Unit` add a hard-filter pass for runtime compatibility and declared resource needs.
- [ ] `Unit` add a scoring pass for admission state, queue saturation, running load, CPU, and RAM signals.
- [ ] `Unit` add deterministic tie-break coverage so ranking is stable for identical inputs.
- [ ] `Unit` add graceful fallback behavior for partially missing telemetry.
- [ ] `Integration` client fetches live node status in parallel before final ranking.
- [ ] `Integration` retry-on-capacity-reject preserves the original task attempt identity.
- [ ] `Implementation` add the live ranking pipeline to remote node selection.
- [ ] `Implementation` add placement-reason strings to user-visible run output.
- [ ] `Implementation` add a grouped no-placement failure summary.
- [ ] `Docs` explain node selection behavior in user terms, not implementation terms.

## Milestone 6: `TASKS.py` Makes Remote Intent Easy To Express

### User Outcome

The user should not need to repeat remote configuration on every task.

They should be able to:

- declare project-level remote defaults once
- opt a task into a named queue without repeating everything else
- ask `tak explain <task>` what will actually happen after inheritance and overrides

### Atomic Tasks

- [ ] `Discovery` run `tak docs dump` and choose the closest shipped example as the baseline for all `TASKS.py` remote-default design work.
- [ ] `BDD` a project can declare a remote default once and tasks inherit it without repeating the same execution block.
- [ ] `BDD` a task can override only the queue name while inheriting the project remote default.
- [ ] `BDD` a task can explicitly opt out of the project remote default and stay local.
- [ ] `BDD` `tak explain <task>` shows the effective remote target, queue, and inherited fields.
- [ ] `BDD` `tak explain <task>` distinguishes explicit task settings from inherited defaults.
- [ ] `Unit` add IR support for project-level execution defaults.
- [ ] `Unit` add merge-rule coverage for task overrides over project defaults.
- [ ] `Unit` add validation coverage for queue names referenced from `TASKS.py`.
- [ ] `Unit` add validation coverage for invalid local-only plus queue-specific remote combinations.
- [ ] `Integration` loader-to-explain preserves inherited defaults end to end.
- [ ] `Integration` docs-dump output includes the new remote-default examples.
- [ ] `Implementation` add project-level remote defaults to `TASKS.py`.
- [ ] `Implementation` add an optional queue selector on the existing remote target surface.
- [ ] `Implementation` add effective-configuration rendering to `tak explain`.
- [ ] `Docs` add one shortest-possible remote-default example and one mixed local/remote example.

## Done Condition

This roadmap is complete only when:

- every checkbox above is complete
- the latest code-change `make check` run is green
- the user can discover node capacity, live CPU/RAM/storage, queue state, and admission behavior from the CLI without reading source code
- a new node can be configured interactively without hand-editing config
- `tak run` behaves better under load than it does today
- `TASKS.py` remote configuration is materially shorter for common cases
