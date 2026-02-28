# large/27_hybrid_local_remote_test_suite_success

## Scenario Goal
Run one part of a test suite locally and one part remotely, then publish one merged report.

Large tier: hybrid execution where local and remote test phases are both required.

## What This Example Exercises
- local test/bootstrap tasks and remote integration task in one dependency chain
- strict `RemoteOnly` execution for the remote test phase
- artifact roundtrip where remote test outputs are consumed by a local summary step

## Runbook
1. `tak list`
2. `tak explain //apps/web:suite_success`
3. `tak graph //apps/web:suite_success --format dot`
4. `tak run //apps/web:suite_success`

## Expected Artifacts
- `out/local-bootstrap.log`
- `out/local-unit.log`
- `out/remote-integration.log`
- `out/remote-junit.txt`
- `out/hybrid-suite-summary.txt`

## Notes
This example is intended for catalog contract execution where
`__TAK_REMOTE_ENDPOINT__` is replaced by a deterministic direct-HTTP fixture endpoint.
