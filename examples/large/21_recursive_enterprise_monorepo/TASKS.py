# Example: large/21_recursive_enterprise_monorepo
# File: TASKS.py
# Scenario: recursive enterprise monorepo

SPEC = module_spec(
  project_id="example_large_21",
  includes=[path("apps/portal"), path("platform/auth"), path("platform/billing")],
  tasks=[task("bootstrap", steps=[cmd("sh", "-c", "mkdir -p out && echo bootstrap >> out/enterprise.log")])]
)
SPEC
