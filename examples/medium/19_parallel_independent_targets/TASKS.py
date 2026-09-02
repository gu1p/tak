# Example: medium/19_parallel_independent_targets
# File: TASKS.py
# Scenario: parallel independent targets

SPEC = module_spec(
    spec_version=2,
  project_id="example_medium_19",
  tasks=[
    task("a", outputs=[path("out/a.txt")], steps=[cmd("sh", "-c", "mkdir -p out && echo a > out/a.txt")]),
    task("b", outputs=[path("out/b.txt")], steps=[cmd("sh", "-c", "mkdir -p out && echo b > out/b.txt")]),
    task("aggregate", deps=[":a", ":b"], outputs=[path("out/parallel.log")], steps=[cmd("sh", "-c", "cat out/a.txt out/b.txt > out/parallel.log && echo aggregate >> out/parallel.log")])
  ]
)
SPEC
