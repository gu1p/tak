# Example: large/21_recursive_enterprise_monorepo
# File: platform/auth/TASKS.py
# Scenario: recursive enterprise monorepo

SPEC = module_spec(
  tasks=[
    task("build", deps=["//:bootstrap"], steps=[cmd("sh", "-c", "echo auth-build >> out/enterprise.log")]),
    task("test", deps=[":build"], steps=[cmd("sh", "-c", "echo auth-test >> out/enterprise.log")]),
  ]
)
SPEC
