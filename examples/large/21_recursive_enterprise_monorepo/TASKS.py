# Example: large/21_recursive_enterprise_monorepo
# File: TASKS.py
# Scenario: recursive enterprise monorepo

SPEC = module_spec(
    spec_version=2,
  project_id="example_large_21",
  includes=[path("apps/portal"), path("platform/auth"), path("platform/billing")],
  tasks=[task(
    "bootstrap",
    outputs=[path("out/bootstrap.txt")],
    steps=[cmd("sh", "-c", "mkdir -p out && echo bootstrap > out/bootstrap.txt")],
  )]
)
SPEC
