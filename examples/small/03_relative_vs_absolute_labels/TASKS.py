# Example: small/03_relative_vs_absolute_labels
# File: TASKS.py
# Scenario: relative vs absolute labels

SPEC = module_spec(
    spec_version=2,
  project_id="example_small_03",
  includes=[path("apps/web")],
  tasks=[
    task("root_prepare", outputs=[path("out/labels.log")], steps=[cmd("sh", "-c", "mkdir -p out && echo root >> out/labels.log")])
  ]
)
SPEC
