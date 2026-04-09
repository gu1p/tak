# Example: large/28_hybrid_local_remote_test_suite_failure_with_logs
# File: TASKS.py
# Scenario: hybrid local + remote test suite (failure path with logs)

SPEC = module_spec(
  project_id="example_large_28",
  includes=[path("apps/web")],
  tasks=[
    task(
      "bootstrap_local",
      steps=[cmd("sh", "-c", "mkdir -p out && echo bootstrap-local-ok > out/local-bootstrap.log")],
    ),
  ]
)
SPEC
