# Example: medium/19_parallel_independent_targets
# File: TASKS.py
# Scenario: parallel independent targets

SPEC = module_spec(
  tasks=[
    task("a", steps=[cmd("sh", "-c", "mkdir -p out && echo a >> out/parallel.log")]),
    task("b", steps=[cmd("sh", "-c", "echo b >> out/parallel.log")]),
    task("aggregate", deps=[":a", ":b"], steps=[cmd("sh", "-c", "echo aggregate >> out/parallel.log")])
  ]
)
SPEC
