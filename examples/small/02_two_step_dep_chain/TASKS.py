# Example: small/02_two_step_dep_chain
# File: TASKS.py
# Scenario: two step dep chain

SPEC = module_spec(
    spec_version=2,
  project_id="example_small_02",
  tasks=[
    task("build", outputs=[path("out/chain.log")], steps=[cmd("sh", "-c", "mkdir -p out && echo build >> out/chain.log")]),
    task("test", deps=[":build"], outputs=[path("out/chain.log")], steps=[cmd("sh", "-c", "mkdir -p out && echo test >> out/chain.log")])
  ]
)
SPEC
