# Example: small/03_relative_vs_absolute_labels
# File: TASKS.py
# Scenario: relative vs absolute labels

SPEC = module_spec(
  tasks=[
    task("root_prepare", steps=[cmd("sh", "-c", "mkdir -p out && echo root >> out/labels.log")])
  ]
)
SPEC
