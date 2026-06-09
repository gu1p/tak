# Test env race: parallel `setenv`/`getenv` in the tak-exec suite

A latent, systemic source of integration-test flakiness. One concrete instance was
fixed (see "Already fixed" below), but the underlying race remains — this note is for
whoever picks up the real fix.

## Symptom

Intermittent failures, only under full-suite parallel load (pass in isolation and on
re-run), where a test gets wrong env-derived config. The one observed so far:

```
infra error: task ... has no configured remote matching pool=Some("build")
  tags=["builder"] capabilities=["linux"] transport=direct
```

i.e. the remote inventory loaded **empty** for a test that had just written its
`remotes.toml` under its own `XDG_CONFIG_HOME`.

## Mechanism

The `tak-exec` integration tests are a single binary (`crates/tak-exec/tests/mod.rs`,
`[[test]] name = "suite"`) running ~99 `#[tokio::test]`s in parallel. Many configure the
code under test by mutating **process environment** (`XDG_CONFIG_HOME`,
`TAKD_REMOTE_EXEC_ROOT`, `PATH`, `TAK_TEST_HOST_PLATFORM`, `TAKD_SOCKET`,
`TAK_REMOTE_WORKSPACE_TRANSFER`) through `EnvGuard` (unsafe `std::env::set_var`),
serialized by a global `env_lock()` in `crates/tak-exec/tests/support/env.rs`.

`std::env::set_var` / `var` are process-global and a **data race / UB** under concurrent
access — Rust 2024 made `set_var` unsafe for exactly this reason, and glibc `setenv` can
reallocate `environ` while another thread is mid-`getenv`.

The hole: `env_lock()` serializes env **writers** against each other, but **readers never
take it**. Roughly 20 of the ~99 tests never call `env_lock`, yet they (and library deps
such as `tempfile` → `TMPDIR`, config-dir resolution → `HOME`/`XDG_*`) call `getenv`
constantly. So a writer's `setenv` (under `env_lock`) runs concurrently with a
non-locking test's `getenv` → the read can return a stale/garbage value. When the
corrupted read is `XDG_CONFIG_HOME`, `default_remote_inventory_path()` resolves the wrong
path and `load_remote_inventory_at` returns an *empty* inventory (a missing file is not an
error), producing "no configured remote matching". Rare; the window widens with more
in-process server activity per test.

## Where to look

- `crates/tak-exec/tests/support/env.rs` — `env_lock`, `EnvGuard`, `LockedEnvGuard`.
- `crates/tak-core/src/remote_inventory.rs` — `default_remote_inventory_path()` (reads
  `XDG_CONFIG_HOME`, ~line 49) and `load_remote_inventory_at` (missing file → empty, ~line 61).
- `crates/tak-exec/src/client_remotes.rs` — `configured_remote_targets` → `inventory_path()`.
- `crates/tak-exec/src/engine/placement_remote.rs:29` — where "no matching" is raised.
- `crates/tak-exec/src/engine/public_types.rs` — `RunOptions` (currently has no
  config/inventory path).

## Fix options

Leaning toward option 1 as the real fix.

1. **Inject config instead of mutating env (cleanest).** Add an optional
   inventory/config-root path to `RunOptions`; thread it through placement to
   `configured_remote_targets` / `default_remote_inventory_path` so it is used when set.
   Tests then pass paths explicitly and stop `setenv`-ing `XDG_CONFIG_HOME` (ideally the
   other per-test vars too), leaving only one-time global `setenv`s. Caveat: placement does
   not currently receive `RunOptions`, so this needs threading — moderately invasive, plus
   ~20 remote test files updated.
2. **Serialize all env-touching tests** (or run the `suite` binary with
   `--test-threads=1`): definitive but a real wall-clock cost. Serializing only writers
   leaves the non-locking readers racing, so it must cover readers too.
3. **Combine:** do option 1 for the hot inventory/config path, then audit and remove the
   remaining per-test `setenv`s.

## Already fixed (do not redo)

The concrete flake observed (commit "test(remote): fix flaky ... env-guard drop order")
was a drop-order bug: `RemoteLeaseCase` stored its `env_lock` guard before its `EnvGuard`,
so on drop the lock released *before* `EnvGuard` restored `XDG_CONFIG_HOME`, clobbering the
next env-locked test. Fixed with `LockedEnvGuard` (restores env *under* the lock; private
fields prevent re-inverting the order), and `RunningTakdServer`'s global env is now set
once via `OnceLock` instead of on every spawn. Those stopped the observed flake but not the
latent reader/writer race described above.

**Rule for new test infra:** never store a `MutexGuard` env lock and an `EnvGuard` as
separate struct/tuple fields with the lock first — use `LockedEnvGuard`.
