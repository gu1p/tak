# Historical: process-global environment races in v1 executor tests

This note records a test failure mode from Tak's removed protocol-v1 client executor. The original
suite configured remote placement by repeatedly changing process-global variables such as
`XDG_CONFIG_HOME`, `TAKD_REMOTE_EXEC_ROOT`, `PATH`, and `TAKD_SOCKET` while tests ran concurrently.
It was possible for one thread to call `setenv` while another read the environment, producing an
empty or incorrect inventory and an intermittent “no configured remote” error.

One drop-order bug was fixed at the time: a test released its environment mutex before restoring
the prior values. Holding the lock through restoration reduced the observed flake, but serializing
writers alone could never make concurrent process-global readers safe.

## Resolution in v2

Execution is now daemon-owned. The `tak` client resolves concrete placement candidates and submits
explicit inputs; it does not select or execute against remotes through the old client path. Client
environment inheritance is also explicit: task/default `pass_env` declarations and command-line
`--pass-env` names are resolved at submission, persisted with owner-only permissions, and delivered
to a worker's otherwise cleared environment.

The v1 executor modules and their environment-mutating remote tests were removed in the coordinated
v2 release, so the old placement race and its proposed client-side remediation no longer apply.

## Still-useful test rule

Process environment remains global to a test binary. New tests should inject paths and values into
the boundary under test whenever possible. If a true environment-boundary test must mutate a
variable, it must hold one guard through both mutation and restoration, and concurrent readers must
participate in the same serialization. Prefer spawning a child process with an explicit environment
when the behavior can be exercised at the CLI boundary.
