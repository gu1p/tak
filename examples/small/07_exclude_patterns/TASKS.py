# Example: small/07_exclude_patterns
# File: TASKS.py
# Scenario: exclude patterns

SPEC = module_spec(
  project_id="example_small_07",
  exclude=["generated/**"],
  tasks=[
    task("main", steps=[cmd("sh", "-c", "mkdir -p out && echo exclude > out/exclude.txt")])
  ]
)
SPEC
