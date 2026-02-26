# Example: medium/18_multi_package_monorepo
# File: apps/web/TASKS.py
# Scenario: multi package monorepo

SPEC = module_spec(
  tasks=[
    task(
      "all",
      deps=["//apps/api:build", "//libs/common:lint"],
      steps=[cmd("sh", "-c", "mkdir -p out && echo web-all >> out/monorepo.log")]
    )
  ]
)
SPEC
