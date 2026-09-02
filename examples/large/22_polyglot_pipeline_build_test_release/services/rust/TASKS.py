# Example: large/22_polyglot_pipeline_build_test_release
# File: services/rust/TASKS.py
# Scenario: polyglot pipeline build test release

SPEC = module_spec(
    spec_version=2,
  tasks=[
    task("build", deps=["//:prepare"], outputs=[path("//out/rust-build.txt")], steps=[cmd("sh", "-c", "mkdir -p out && echo rust-build > out/rust-build.txt", cwd="//")]),
    task("test", deps=[":build"], outputs=[path("//out/rust-test.txt")], steps=[cmd("sh", "-c", "mkdir -p out && echo rust-test > out/rust-test.txt", cwd="//")]),
  ]
)
SPEC
