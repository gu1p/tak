# Example: medium/20_failure_no_retry_unmatched_exit
# File: TASKS.py
# Scenario: failure no retry unmatched exit

SPEC = module_spec(
  project_id="example_medium_20",
  tasks=[
    task(
      "failing",
      retry=retry(attempts=2, on_exit=[2], backoff=fixed(0)),
      steps=[cmd("sh", "-c", "mkdir -p out && exit 3")]
    )
  ]
)
SPEC
