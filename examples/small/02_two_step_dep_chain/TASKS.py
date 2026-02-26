# Example: small/02_two_step_dep_chain
# File: TASKS.py
# Scenario: two step dep chain

SPEC = module_spec(
  tasks=[
    task("build", steps=[cmd("sh", "-c", "mkdir -p out && echo build >> out/chain.log")]),
    task("test", deps=[":build"], steps=[cmd("sh", "-c", "echo test >> out/chain.log")])
  ]
)
SPEC
