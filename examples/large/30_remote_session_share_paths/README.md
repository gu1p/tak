# large/30_remote_session_share_paths

## Why This Matters

Use `SessionReuse.Paths` when remote tasks should stay isolated except for explicit private CAS
cache paths. This example models a Rust/Cargo pipeline where `target/` is reused between remote
build and test tasks. Cache paths are not result artifacts, so both tasks also declare real output
markers.

## Runbook

Bootstrap a direct remote agent, then run:

```bash
tak run //:cargo_test
```

## Expected Signals

- Run summary includes `session=cargo-cache`.
- Run details identify the private paths cache.
- `cargo_test` observes `target/debug/app` from `cargo_build`.

## Artifacts

- `out/build-marker.txt`
- `out/test-marker.txt`
