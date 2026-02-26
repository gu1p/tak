# Example: large/21_recursive_enterprise_monorepo
# File: platform/billing/TASKS.py
# Scenario: recursive enterprise monorepo

SPEC = module_spec(
  tasks=[
    task("build", deps=["//:bootstrap"], steps=[cmd("sh", "-c", "mkdir -p out && echo billing-build >> out/enterprise.log")]),
    task("test", deps=[":build"], steps=[cmd("sh", "-c", "mkdir -p out && echo billing-test >> out/enterprise.log")]),
  ]
)
SPEC
