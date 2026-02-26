# Example: large/22_polyglot_pipeline_build_test_release
# File: services/python/TASKS.py
# Scenario: polyglot pipeline build test release

SPEC = module_spec(
  tasks=[
    task("build", deps=["//:prepare"], steps=[cmd("sh", "-c", "mkdir -p out && echo py-build >> out/polyglot.log")]),
    task("test", deps=[":build"], steps=[cmd("sh", "-c", "mkdir -p out && echo py-test >> out/polyglot.log")]),
    task("release", deps=[":test", "//services/rust:test", "//services/js:test"], steps=[script("scripts/release.sh", interpreter="sh")]),
  ]
)
SPEC
