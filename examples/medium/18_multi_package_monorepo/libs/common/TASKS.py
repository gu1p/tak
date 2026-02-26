# Example: medium/18_multi_package_monorepo
# File: libs/common/TASKS.py
# Scenario: multi package monorepo

SPEC = module_spec(
  tasks=[task("lint", deps=["//:bootstrap"], steps=[cmd("sh", "-c", "mkdir -p out && echo common-lint >> out/monorepo.log")])]
)
SPEC
