# small/04_cmd_with_env_and_cwd

## Why This Matters

Most build and test instability comes from implicit shell context. This example shows explicit `cwd` and `env` so task behavior is deterministic across laptops and CI runners.

## Copy-Paste Starter

```python
SPEC = module_spec(
    tasks=[
        task(
            "env_cmd",
            steps=[
                cmd("mkdir", "-p", "out"),
                cmd(
                    "sh",
                    "-c",
                    "echo \"$TAK_ENV_MARKER\" > marker.txt",
                    cwd="out",
                    env={"TAK_ENV_MARKER": "ENV_OK"},
                ),
            ],
        )
    ]
)
SPEC
```

## Parameter Alternatives

| Parameter | Current value | Alternatives | Behavior impact |
|---|---|---|---|
| `cwd` | `"out"` | omit `cwd` | Step runs from task working root instead of `out/`. |
| `env` | explicit map | inherited shell env only | Less explicit and easier to break across environments. |
| step kind | `cmd(...)` | `script(...)` | Better for long scripts and reuse across tasks. |

## Runbook

1. `tak list`
2. `tak explain //:env_cmd`
3. `tak graph //:env_cmd --format dot`
4. `tak run //:env_cmd`

## Expected Signals

- `tak run` reports success for `env_cmd`.
- The marker value is written from task-scoped environment.

## Artifacts

- `out/marker.txt`
