# Example: medium/18_multi_package_monorepo
# File: TASKS.py
# Scenario: multi package monorepo

SPEC = module_spec(
  tasks=[task("bootstrap", steps=[cmd("sh", "-c", "mkdir -p out && echo bootstrap >> out/monorepo.log")])]
)
SPEC
