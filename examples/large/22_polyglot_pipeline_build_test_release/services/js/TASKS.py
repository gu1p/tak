# Example: large/22_polyglot_pipeline_build_test_release
# File: services/js/TASKS.py
# Scenario: polyglot pipeline build test release

SPEC = module_spec(
    spec_version=2,
  tasks=[
    task("build", deps=["//:prepare"], outputs=[path("//out/js-build.txt")], steps=[cmd("sh", "-c", "mkdir -p out && echo js-build > out/js-build.txt", cwd="//")]),
    task("test", deps=[":build"], outputs=[path("//out/js-test.txt")], steps=[cmd("sh", "-c", "mkdir -p out && echo js-test > out/js-test.txt", cwd="//")]),
  ]
)
SPEC
