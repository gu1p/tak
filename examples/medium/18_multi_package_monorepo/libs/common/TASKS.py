# Example: medium/18_multi_package_monorepo
# File: libs/common/TASKS.py
# Scenario: multi package monorepo

SPEC = module_spec(
    spec_version=2,
  tasks=[task(
    "lint",
    deps=["//:bootstrap"],
    outputs=[path("//out/common-lint.txt")],
    steps=[cmd("sh", "-c", "mkdir -p out && echo common-lint > out/common-lint.txt", cwd="//")],
  )]
)
SPEC
