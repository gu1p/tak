# medium/18_multi_package_monorepo

## Why This Matters

This is the first example that looks like a real monorepo: root bootstrap tasks, app tasks, and shared library tasks, all resolved as one graph.

## Copy-Paste Starter

```python
# TASKS.py
SPEC = module_spec(
    project_id="example_medium_18",
    includes=[path("apps/api"), path("apps/web"), path("libs/common")],
    tasks=[task("bootstrap", steps=[cmd("sh", "-c", "mkdir -p out && echo bootstrap >> out/monorepo.log")])],
)
SPEC
```

Included package modules can keep their own task files:

```python
# apps/web/TASKS.py
SPEC = module_spec(
    tasks=[
        task(
            "all",
            deps=["//apps/api:build", "//libs/common:lint"],
            steps=[cmd("sh", "-c", "mkdir -p out && echo web-all >> out/monorepo.log")],
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
| output strategy | single `out/monorepo.log` | per-package output files | Per-package outputs simplify ownership and debugging. |

## Runbook

1. `tak list`
2. `tak explain //apps/web:all`
3. `tak graph //apps/web:all --format dot`
4. `tak run //apps/web:all`

## Expected Signals

- The graph includes root bootstrap plus `apps/api` and `libs/common` dependencies.
- `tak run` executes prerequisites before `apps/web:all`.

## Artifacts

- `out/monorepo.log`
