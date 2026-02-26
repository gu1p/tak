# Example: medium/18_multi_package_monorepo
# File: apps/api/TASKS.py
# Scenario: multi package monorepo

SPEC = module_spec(
  tasks=[task("build", deps=["//:bootstrap"], steps=[cmd("sh", "-c", "echo api-build >> out/monorepo.log")])]
)
SPEC
