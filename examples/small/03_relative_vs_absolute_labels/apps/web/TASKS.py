# Example: small/03_relative_vs_absolute_labels
# File: apps/web/TASKS.py
# Scenario: relative vs absolute labels

SPEC = module_spec(
    spec_version=2,
  tasks=[
    task("build", deps=["//:root_prepare"], outputs=[path("//out/labels.log")], steps=[cmd("sh", "-c", "mkdir -p out && echo web-build >> out/labels.log", cwd="//")]),
    task("test", deps=[":build"], outputs=[path("//out/labels.log")], steps=[cmd("sh", "-c", "mkdir -p out && echo web-test >> out/labels.log", cwd="//")])
  ]
)
SPEC
