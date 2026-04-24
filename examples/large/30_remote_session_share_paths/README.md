# large/30_remote_session_share_paths

## Why This Matters

Use `SharePaths` when remote tasks should stay isolated except for explicit cache directories. This example models a Rust/Cargo pipeline where `target/` is reused between remote build and test tasks.

## Runbook

Bootstrap a direct remote agent, then run:

```bash
tak run //:cargo_test
```

## Expected Signals

- Run summary includes `session=cargo-cache`.
- Run summary includes `reuse=share_paths`.
- `cargo_test` observes `target/debug/app` from `cargo_build`.

## Artifacts

- `out/build-marker.txt`
- `out/test-marker.txt`
