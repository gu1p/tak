# Example: medium/18_multi_package_monorepo
# File: TASKS.py
# Scenario: multi package monorepo

SPEC = module_spec(
    spec_version=2,
  project_id="example_medium_18",
  includes=[path("apps/api"), path("apps/web"), path("libs/common")],
  tasks=[task(
    "bootstrap",
    outputs=[path("out/bootstrap.txt")],
    steps=[cmd("sh", "-c", "mkdir -p out && echo bootstrap > out/bootstrap.txt")],
  )]
)
SPEC
