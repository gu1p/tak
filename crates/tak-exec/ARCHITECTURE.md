# tak-exec Architecture

## Purpose

`tak-exec` contains process, container, and remote execution primitives used behind daemon worker
boundaries. It is not the `tak` CLI's v2 scheduler or fallback executor.

For v2, `tak` resolves a run and submits it to local `takd`. The daemon owns graph scheduling,
retry/cancellation policy, placement, attempts, and persisted events. Local and remote worker paths
may reuse `tak-exec` primitives to launch concrete steps and collect declared outputs.

## Worker responsibilities

- launch command and script steps with explicit cwd and literal environment values;
- execute in local host or container runtimes selected by the daemon;
- enforce the attempt timeout/cancellation signal supplied by the coordinator;
- stream stdout/stderr to a daemon-owned observer;
- stage workspace context and declared outputs;
- support direct/Tor worker transport helpers where retained by the daemon; and
- classify process/runtime failures without deciding graph-level retries.

## Boundary rules

- No public CLI execution command may call `tak-exec` as a client-side fallback.
- No worker primitive owns dependency readiness, queue fairness, limiter admission, or node
  selection.
- Output selectors remain workspace-relative and are validated again before daemon artifact commit.
- Environment values come from literal step `env` plus the resolved `pass_env` allowlist; workers
  do not inherit arbitrary client environment.
- Cancellation and attempt generations come from takd. A worker result is evidence, not authority
  to settle a stale attempt.

## Legacy surface

Some v1 executor types and tests remain during the coordinated v2 transition. They are internal
compatibility code, not an authorization to bypass local takd. Public version mismatch and missing
daemon paths fail with upgrade/start guidance and no legacy fallback.

## Main areas

- step/process runners and output observers;
- container runtime planning and lifecycle;
- context staging and output collection;
- content hashing and private cache helpers; and
- remote transport helpers used by takd workers.
