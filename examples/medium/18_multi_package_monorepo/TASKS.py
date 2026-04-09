# Example: medium/18_multi_package_monorepo
# File: TASKS.py
# Scenario: multi package monorepo

SPEC = module_spec(
  project_id="example_medium_18",
  includes=[path("apps/api"), path("apps/web"), path("libs/common")],
  tasks=[task("bootstrap", steps=[cmd("sh", "-c", "mkdir -p out && echo bootstrap >> out/monorepo.log")])]
)
SPEC
