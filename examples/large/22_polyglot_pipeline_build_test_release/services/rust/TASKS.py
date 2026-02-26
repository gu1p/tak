# Example: large/22_polyglot_pipeline_build_test_release
# File: services/rust/TASKS.py
# Scenario: polyglot pipeline build test release

SPEC = module_spec(
  tasks=[
    task("build", deps=["//:prepare"], steps=[cmd("sh", "-c", "echo rust-build >> out/polyglot.log")]),
    task("test", deps=[":build"], steps=[cmd("sh", "-c", "echo rust-test >> out/polyglot.log")]),
  ]
)
SPEC
