# Example: small/07_exclude_patterns
# File: TASKS.py
# Scenario: exclude patterns

SPEC = module_spec(
    spec_version=2,
  project_id="example_small_07",
  exclude=["generated/**"],
  tasks=[
    task("main", outputs=[path("out/exclude.txt")], steps=[cmd("sh", "-c", "mkdir -p out && echo exclude > out/exclude.txt")])
  ]
)
SPEC
