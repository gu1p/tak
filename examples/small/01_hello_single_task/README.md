# small/01_hello_single_task

## Why This Matters

This is the minimum useful Tak setup: one task, one step, one artifact. Use it as a baseline template before layering retries, needs, or remote execution.

## Copy-Paste Starter

```python
SPEC = module_spec(
    tasks=[
        task(
            "hello",
            doc="Writes a hello output file.",
            steps=[cmd("sh", "-c", "mkdir -p out && echo hello > out/hello.txt")],
            tags=["starter"],
        )
    ]
)
SPEC
```

## Parameter Alternatives

| Parameter | Current value | Alternatives | Behavior impact |
|---|---|---|---|
| `steps` | `cmd("sh", "-c", "...")` | `script("scripts/build.sh", interpreter="sh")` | Move complex shell logic to a reusable script file. |
| `tags` | `["starter"]` | `[]` or custom tags | Helps filtering/grouping tasks in larger repos. |
| `doc` | short description | richer runbook text | Improves explainability when sharing task intent. |

## Runbook

1. `tak list`
2. `tak explain //:hello`
3. `tak graph //:hello --format dot`
4. `tak run //:hello`

## Expected Signals

- `tak list` includes `//:hello` and the authored `Writes a hello output file.` description.
- `tak run` prints a success summary for `hello`.
- Exit code is zero.

## Artifacts

- `out/hello.txt`
