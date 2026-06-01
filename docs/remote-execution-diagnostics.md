# Remote Execution Diagnostics

## Before

Tak used to expose the local daemon placement placeholder as if it were a
worker:

```text
//:check [attempt 1] probing remote node __takd_daemon_tor__
//:check [attempt 1] connected to remote node __takd_daemon_tor__
Error: infra error: local takd daemon rejected request ... while contacting remote node __takd_daemon_tor__: no Tor peers have enough resource capacity
```

That wording was wrong: `__takd_daemon_tor__` is the local `takd` placement
and Tor relay path, not a remote worker.

## After

Before a worker is selected, status is local-daemon / relay scoped:

```text
//:check [attempt 1] connecting to local takd daemon
//:check [attempt 1] local takd: discovering remote capacity over Tor
//:check [attempt 1] upload [----------] 0% 3.77 MB through local takd Tor relay
```

Once the daemon returns a real worker, status names that worker:

```text
//:check [attempt 1] upload 100% 3.77/3.77 MB to remote node node-2c852b72784c463d93d39e3a5118cdc4
//:check [attempt 1] remote worker node-2c852b72784c463d93d39e3a5118cdc4 selected by local takd; task accepted
//:check [attempt 1] queued: waiting for remote capacity (queue position: 3; 2 tasks ahead)
```

Impossible capacity errors are explicit:

```text
Error: local takd could not place this task on a Tor remote worker

subsystem: placement
stage: remote placement
transport: tor
retryable: no
original_error:
No known remote worker satisfies this task's requirements.

Task requires:
  cpu: 16.00

largest known worker:
  cpu: 8.00
  memory: 16384 MB

This task cannot run until a larger worker joins the network or its requirements are reduced.
source: crates/takd/src/daemon/peer_manager/eligibility.rs
```
