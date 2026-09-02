# Example: medium/18_multi_package_monorepo
# File: apps/web/TASKS.py
# Scenario: multi package monorepo

SPEC = module_spec(
    spec_version=2,
  tasks=[
    task(
      "all",
      deps=["//apps/api:build", "//libs/common:lint"],
      outputs=[path("//out/monorepo.log")],
      steps=[cmd(
        "sh", "-c",
        "cat out/bootstrap.txt out/api-build.txt out/common-lint.txt > out/monorepo.log && echo web-all >> out/monorepo.log",
        cwd="//",
      )]
    )
  ]
)
SPEC
