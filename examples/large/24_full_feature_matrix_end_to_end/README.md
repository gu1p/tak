# large/24_full_feature_matrix_end_to_end

## Why This Matters

This scenario composes nearly every major Tak primitive in one realistic release flow: defaults, retries, scoped needs, queue priority, scripts, and recursive packages.

## Copy-Paste Starter

```python
SPEC = module_spec(
    limiters=[
        resource("cpu", 8, unit="slots", scope=MACHINE),
        lock("ui_lock", scope=MACHINE),
        rate_limit("start_rl", burst=5, refill_per_second=10, scope=MACHINE),
    ],
    queues=[queue_def("qa_priority", slots=1, discipline=PRIORITY, scope=MACHINE)],
    defaults={"retry": retry(attempts=2, on_exit=[44], backoff=fixed(0))},
    tasks=[
        task(
            "validate",
            needs=[
                need("cpu", 2, scope=MACHINE),
                need("ui_lock", 1, scope=MACHINE),
                need("start_rl", 1, scope=MACHINE, hold=AT_START),
            ],
            queue=queue_use("qa_priority", scope=MACHINE, slots=1, priority=10),
            steps=[cmd("sh", "-c", "mkdir -p out && echo qa-validate >> out/full_matrix.log")],
        ),
        task(
            "release",
            deps=[":validate"],
            steps=[script("scripts/matrix_release.sh", interpreter="sh")],
        ),
    ],
)
SPEC
```

## Parameter Alternatives

| Parameter | Current value | Alternatives | Behavior impact |
|---|---|---|---|
| queue discipline | `PRIORITY` | `FIFO` | `PRIORITY` enables urgent tasks to jump ahead of default traffic. |
| limiter scope | mostly `MACHINE` | `USER`, `PROJECT`, `WORKTREE` | Choose where contention is isolated. |
| `defaults.retry` | shared retry default | task-specific retry overrides | Keeps global policy consistent while allowing targeted exceptions. |
| `hold` | `AT_START` for rate limiter | `DURING` | `AT_START` can reduce long token hold time for admission-style limits. |

## Runbook

1. `tak list`
2. `tak explain //apps/qa:release`
3. `tak graph //apps/qa:release --format dot`
4. `tak run //apps/qa:release`

## Expected Signals

- `tak run` succeeds after a full dependency chain.
- Summary lines include retry and placement metadata for each executed task.

## Artifacts

- `out/full_matrix_release.txt`
