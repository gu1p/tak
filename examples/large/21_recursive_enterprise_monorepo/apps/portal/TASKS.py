# Example: large/21_recursive_enterprise_monorepo
# File: apps/portal/TASKS.py
# Scenario: recursive enterprise monorepo

SPEC = module_spec(
    spec_version=2,
  tasks=[
    task(
      "release",
      deps=["//platform/auth:test", "//platform/billing:test"],
      outputs=[path("//out/enterprise.log")],
      steps=[cmd(
        "sh", "-c",
        "cat out/bootstrap.txt out/auth-build.txt out/auth-test.txt out/billing-build.txt out/billing-test.txt > out/enterprise.log && echo portal-release >> out/enterprise.log",
        cwd="//",
      )]
    )
  ]
)
SPEC
