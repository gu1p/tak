# large/25_remote_direct_build_and_artifact_roundtrip

## Scenario Goal
Run a build remotely, then consume the synced artifact locally in the release flow.

Large tier: remote orchestration and artifact handoff at workspace scale.

## What This Example Exercises
- strict `RemoteOnly` remote execution over direct endpoint
- remote output sync back to local workspace
- local follow-up verification using remote artifact

## Runbook
1. `tak list`
2. `tak explain //services/api:release`
3. `tak graph //services/api:release --format dot`
4. `tak run //services/api:release`

## Expected Artifacts
- `out/remote-build-artifact.txt`
- `out/remote-build.log`
- `out/local-verify.log`
- `out/release-summary.txt`

## Notes
This example is intended for catalog contract execution where
`__TAK_REMOTE_ENDPOINT__` is replaced by a deterministic test fixture endpoint.
