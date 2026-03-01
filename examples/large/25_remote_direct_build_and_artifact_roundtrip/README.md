# large/25_remote_direct_build_and_artifact_roundtrip

## Why This Matters

This is the core remote-delivery pattern: build remotely, sync artifacts back, then verify and release locally.

## Copy-Paste Starter

```python
REMOTE = Remote(id="remote-direct-build", endpoint="https://remote.example")

SPEC = module_spec(
    tasks=[
        task(
            "build_remote",
            steps=[
                cmd(
                    "sh",
                    "-c",
                    "mkdir -p out && "
                    "echo artifact-from-remote-build > out/remote-build-artifact.txt && "
                    "echo remote-build-ok > out/remote-build.log",
                )
            ],
            execution=RemoteOnly(REMOTE),
        ),
        task(
            "verify_artifact",
            deps=[":build_remote"],
            steps=[
                cmd(
                    "sh",
                    "-c",
                    "grep -q artifact-from-remote-build out/remote-build-artifact.txt && "
                    "echo verify-local-ok > out/local-verify.log",
                )
            ],
        ),
    ]
)
SPEC
```

## Parameter Alternatives

| Parameter | Current value | Alternatives | Behavior impact |
|---|---|---|---|
| execution mode | `RemoteOnly(REMOTE)` | `LocalOnly(Local(...))`, `ByCustomPolicy(...)` | Force remote, force local, or pick dynamically with policy logic. |
| remote transport | direct endpoint | Tor onion transport configuration | Enables private-network routing with transport parity. |
| remote runtime | default runtime | `Remote(..., runtime=ContainerRuntime(...))` | Containerized remote runtime gives stronger environment reproducibility. |

## Runbook

1. `tak list`
2. `tak explain //services/api:release`
3. `tak graph //services/api:release --format dot`
4. `tak run //services/api:release`

## Expected Signals

- Run summary includes `placement=remote` for the remote build task.
- Run summary includes `remote_node=remote-direct-build`.
- Local verify step succeeds using remote-generated artifact.

## Artifacts

- `out/local-context.txt`
- `out/remote-build-artifact.txt`
- `out/remote-build.log`
- `out/local-verify.log`
- `out/release-summary.txt`
