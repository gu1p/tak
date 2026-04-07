# medium/11_machine_lock_shared_ui

## Why This Matters

UI and device-bound tests often cannot run in parallel safely. This example shows machine-scoped locking via daemon leases so concurrent runs do not fight for the same shared resource.

## Copy-Paste Starter

```python
SPEC = module_spec(
    limiters=[lock("ui_lock", scope=MACHINE)],
    tasks=[
        task(
            "ui_test",
            needs=[need("ui_lock", 1, scope=MACHINE)],
            steps=[cmd("sh", "-c", "mkdir -p out && echo ui-lock > out/ui_lock.txt")],
        )
    ],
)
SPEC
```

## Parameter Alternatives

| Parameter | Current value | Alternatives | Behavior impact |
|---|---|---|---|
| limiter kind | `lock("ui_lock")` | `resource(...)`, `rate_limit(...)`, `process_cap(...)` | Switch between exclusive lock, capacity pool, token rate, or process match guard. |
| `scope` | `MACHINE` | `USER`, `PROJECT`, `WORKTREE` | Changes the contention boundary and fairness domain. |
| `hold` in `need(...)` | default `DURING` | `AT_START` | `AT_START` acquires at start and releases earlier after admission. |

## Runbook

1. `tak list`
2. `tak explain //:ui_test`
3. `tak graph //:ui_test --format dot`
4. `tak run //:ui_test`

## Expected Signals

- `tak run` succeeds with local coordination available.
- `tak status` is currently unsupported in the client-only CLI build.

## Artifacts

- `out/ui_lock.txt`
