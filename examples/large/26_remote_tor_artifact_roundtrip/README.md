# large/26_remote_tor_artifact_roundtrip

## Scenario Goal
Run a task remotely via Tor transport and consume synced artifacts locally.

Large tier: remote transport parity and artifact roundtrip flow.

## What This Example Exercises
- `Transport.TorOnionService()`
- remote execution over Tor transport abstraction
- local consumption of remote-produced artifacts

## Runbook
1. `tak list`
2. `tak explain //:consume_remote_report`
3. `tak graph //:consume_remote_report --format dot`
4. `tak run //:consume_remote_report`

## Expected Artifacts
- `out/tor-remote-artifact.txt`
- `out/tor-remote.log`
- `out/tor-roundtrip.txt`

## Notes
This example is intended for catalog contract execution where
client remote inventory is pre-seeded with a deterministic Tor fixture agent.
