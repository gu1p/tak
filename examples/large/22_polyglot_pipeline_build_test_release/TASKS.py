# Example: large/22_polyglot_pipeline_build_test_release
# File: TASKS.py
# Scenario: polyglot pipeline build test release

SPEC = module_spec(
    spec_version=2,
  project_id="example_large_22",
  includes=[path("services/js"), path("services/python"), path("services/rust")],
  tasks=[task(
    "prepare",
    outputs=[path("out/prepare.txt")],
    steps=[cmd("sh", "-c", "mkdir -p out && echo prepare > out/prepare.txt")],
  )]
)
SPEC
