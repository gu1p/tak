# small/08_retry_fixed_fail_once

## Why This Matters

Flaky external steps are common. This example shows deterministic recovery by retrying only specific exit codes with a fixed backoff.

## Copy-Paste Starter

```python
SPEC = module_spec(
    tasks=[
        task(
            "flaky_fixed",
            retry=retry(attempts=2, on_exit=[42], backoff=fixed(0)),
            steps=[
                cmd(
                    "sh",
                    "-c",
                    "mkdir -p out && "
                    "if [ -f out/seen_fixed ]; then "
                    "echo recovered > out/retry_fixed.txt; exit 0; "
                    "else touch out/seen_fixed; exit 42; fi",
                )
            ],
        )
    ]
)
SPEC
```

## Parameter Alternatives

| Parameter | Current value | Alternatives | Behavior impact |
|---|---|---|---|
| `attempts` | `2` | `1`, `3+` | `1` disables retries; higher values tolerate more transient failures. |
| `on_exit` | `[42]` | `[]` or multiple codes | `[]` retries any non-zero; explicit list limits retries to known transient codes. |
| `backoff` | `fixed(0)` | `fixed(1.5)`, `exp_jitter(min_s=1, max_s=60)` | Longer or jittered delays reduce thundering-herd retries. |

## Runbook

1. `tak list`
2. `tak explain //:flaky_fixed`
3. `tak graph //:flaky_fixed --format dot`
4. `tak run //:flaky_fixed`

## Expected Signals

- Run succeeds even though the first attempt exits `42`.
- `tak run` summary shows `attempts=2` for `flaky_fixed`.

## Artifacts

- `out/retry_fixed.txt`
