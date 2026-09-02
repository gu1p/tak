# Example: large/22_polyglot_pipeline_build_test_release
# File: services/python/TASKS.py
# Scenario: polyglot pipeline build test release

SPEC = module_spec(
    spec_version=2,
  tasks=[
    task("build", deps=["//:prepare"], outputs=[path("//out/python-build.txt")], steps=[cmd("sh", "-c", "mkdir -p out && echo py-build > out/python-build.txt", cwd="//")]),
    task("test", deps=[":build"], outputs=[path("//out/python-test.txt")], steps=[cmd("sh", "-c", "mkdir -p out && echo py-test > out/python-test.txt", cwd="//")]),
    task("release", deps=[":test", "//services/rust:test", "//services/js:test"], outputs=[path("//out/polyglot_release.txt")], steps=[script("//scripts/release.sh", interpreter="sh", cwd="//")]),
  ]
)
SPEC
