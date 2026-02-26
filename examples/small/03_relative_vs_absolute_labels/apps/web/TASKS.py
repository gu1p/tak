# Example: small/03_relative_vs_absolute_labels
# File: apps/web/TASKS.py
# Scenario: relative vs absolute labels

SPEC = module_spec(
  tasks=[
    task("build", deps=["//:root_prepare"], steps=[cmd("sh", "-c", "echo web-build >> out/labels.log")]),
    task("test", deps=[":build"], steps=[cmd("sh", "-c", "echo web-test >> out/labels.log")])
  ]
)
SPEC
