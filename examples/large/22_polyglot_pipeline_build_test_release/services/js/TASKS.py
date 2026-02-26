# Example: large/22_polyglot_pipeline_build_test_release
# File: services/js/TASKS.py
# Scenario: polyglot pipeline build test release

SPEC = module_spec(
  tasks=[
    task("build", deps=["//:prepare"], steps=[cmd("sh", "-c", "mkdir -p out && echo js-build >> out/polyglot.log")]),
    task("test", deps=[":build"], steps=[cmd("sh", "-c", "mkdir -p out && echo js-test >> out/polyglot.log")]),
  ]
)
SPEC
