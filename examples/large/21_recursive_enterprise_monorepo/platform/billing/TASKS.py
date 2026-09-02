# Example: large/21_recursive_enterprise_monorepo
# File: platform/billing/TASKS.py
# Scenario: recursive enterprise monorepo

SPEC = module_spec(
    spec_version=2,
  tasks=[
    task(
      "build",
      deps=["//:bootstrap"],
      outputs=[path("//out/billing-build.txt")],
      steps=[cmd("sh", "-c", "mkdir -p out && echo billing-build > out/billing-build.txt", cwd="//")],
    ),
    task(
      "test",
      deps=[":build"],
      outputs=[path("//out/billing-test.txt")],
      steps=[cmd("sh", "-c", "mkdir -p out && echo billing-test > out/billing-test.txt", cwd="//")],
    ),
  ]
)
SPEC
