# Example: medium/19_parallel_independent_targets
# File: TASKS.py
# Scenario: parallel independent targets

SPEC = module_spec(
  project_id="example_medium_19",
  tasks=[
    task("a", steps=[cmd("sh", "-c", "mkdir -p out && echo a >> out/parallel.log")]),
    task("b", steps=[cmd("sh", "-c", "mkdir -p out && echo b >> out/parallel.log")]),
    task("aggregate", deps=[":a", ":b"], steps=[cmd("sh", "-c", "mkdir -p out && echo aggregate >> out/parallel.log")])
  ]
)
SPEC
