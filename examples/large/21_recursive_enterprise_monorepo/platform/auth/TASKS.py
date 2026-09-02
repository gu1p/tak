# Example: large/21_recursive_enterprise_monorepo
# File: platform/auth/TASKS.py
# Scenario: recursive enterprise monorepo

SPEC = module_spec(
    spec_version=2,
  tasks=[
    task(
      "build",
      deps=["//:bootstrap"],
      outputs=[path("//out/auth-build.txt")],
      steps=[cmd("sh", "-c", "mkdir -p out && echo auth-build > out/auth-build.txt", cwd="//")],
    ),
    task(
      "test",
      deps=[":build"],
      outputs=[path("//out/auth-test.txt")],
      steps=[cmd("sh", "-c", "mkdir -p out && echo auth-test > out/auth-test.txt", cwd="//")],
    ),
  ]
)
SPEC
