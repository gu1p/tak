# Example: large/21_recursive_enterprise_monorepo
# File: apps/portal/TASKS.py
# Scenario: recursive enterprise monorepo

SPEC = module_spec(
  tasks=[
    task(
      "release",
      deps=["//platform/auth:test", "//platform/billing:test"],
      steps=[cmd("sh", "-c", "mkdir -p out && echo portal-release >> out/enterprise.log")]
    )
  ]
)
SPEC
