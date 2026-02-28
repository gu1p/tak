# large/28_hybrid_local_remote_test_suite_failure_with_logs

## Scenario Goal
Run local checks first, then run a remote test phase that fails and returns failure artifacts.

Large tier: hybrid execution with explicit remote failure diagnostics.

## What This Example Exercises
- local + remote split in one suite pipeline
- strict `RemoteOnly` test execution for the failing remote suite phase
- synced failure artifacts (`out/remote-test-output.log` and `out/remote-failure-reason.txt`)
- run failure contract while still preserving remote diagnostics for local inspection

## Runbook
1. `tak list`
2. `tak explain //apps/web:remote_suite`
3. `tak graph //apps/web:remote_suite --format dot`
4. `tak run //apps/web:remote_suite`

## Expected Behavior
- command exits non-zero
- stderr includes task failure reason
- failure logs remain available in `out/`

## Expected Artifacts
- `out/local-bootstrap.log`
- `out/local-unit.log`
- `out/remote-test-output.log`
- `out/remote-failure-reason.txt`

## Notes
This example is intended for catalog contract execution where
`__TAK_REMOTE_ENDPOINT__` is replaced by a deterministic direct-HTTP fixture endpoint.
