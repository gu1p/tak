# Example: medium/18_multi_package_monorepo
# File: apps/api/TASKS.py
# Scenario: multi package monorepo

SPEC = module_spec(
    spec_version=2,
  tasks=[task(
    "build",
    deps=["//:bootstrap"],
    outputs=[path("//out/api-build.txt")],
    steps=[cmd("sh", "-c", "mkdir -p out && echo api-build > out/api-build.txt", cwd="//")],
  )]
)
SPEC
