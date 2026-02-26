# Example: large/21_recursive_enterprise_monorepo
# File: TASKS.py
# Scenario: recursive enterprise monorepo

SPEC = module_spec(
  tasks=[task("bootstrap", steps=[cmd("sh", "-c", "mkdir -p out && echo bootstrap >> out/enterprise.log")])]
)
SPEC
