# Example: large/27_hybrid_local_remote_test_suite_success
# File: TASKS.py
# Scenario: hybrid local + remote test suite (success path)

SPEC = module_spec(
    spec_version=2,
  project_id="example_large_27",
  includes=[path("apps/web")],
  tasks=[
    task(
      "bootstrap_local",
      outputs=[path("out/local-bootstrap.log")],
      steps=[cmd("sh", "-c", "mkdir -p out && echo bootstrap-local-ok > out/local-bootstrap.log")],
    ),
  ]
)
SPEC
