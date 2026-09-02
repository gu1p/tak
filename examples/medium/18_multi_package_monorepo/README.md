# medium/18_multi_package_monorepo

## Why This Matters

This is the first example that looks like a real monorepo: root bootstrap tasks, app tasks, and shared library tasks, all resolved as one graph.

## Copy-Paste Starter

```python
# TASKS.py
SPEC = module_spec(
    spec_version=2,
    project_id="example_medium_18",
    includes=[path("apps/api"), path("apps/web"), path("libs/common")],
    tasks=[task(
        "bootstrap",
        outputs=[path("out/bootstrap.txt")],
        steps=[cmd("sh", "-c", "mkdir -p out && echo bootstrap > out/bootstrap.txt")],
    )],
)
SPEC
```

Included package modules can keep their own task files:

```python
# apps/web/TASKS.py
SPEC = module_spec(
    spec_version=2,
    tasks=[
        task(
            "all",
            deps=["//apps/api:build", "//libs/common:lint"],
            outputs=[path("//out/monorepo.log")],
            steps=[cmd(
                "sh", "-c",
                "cat out/bootstrap.txt out/api-build.txt out/common-lint.txt > out/monorepo.log && echo web-all >> out/monorepo.log",
                cwd="//",
            )],
        )
    ]
)
SPEC
```

## Parameter Alternatives

| Parameter | Current value | Alternatives | Behavior impact |
|---|---|---|---|
| dependency labels | absolute labels (`//apps/api:build`) | relative labels (`:build`) where appropriate | Absolute labels make cross-package intent explicit and stable. |
| topology | app depends on shared + api | fan-out from root bootstrap | Lets you control bottlenecks and critical path shape. |
| output strategy | per-package outputs plus final `out/monorepo.log` | one final task per report | Unique branch outputs avoid ambiguous independent writes; the fan-in task owns aggregation. |

## Runbook

1. `tak list`
2. `tak explain //apps/web:all`
3. `tak graph //apps/web:all --format dot`
4. `tak run //apps/web:all`

## Expected Signals

- The graph includes root bootstrap plus `apps/api` and `libs/common` dependencies.
- `tak run` executes prerequisites before `apps/web:all`.

## Artifacts

- `out/bootstrap.txt`
- `out/api-build.txt`
- `out/common-lint.txt`
- `out/monorepo.log`
