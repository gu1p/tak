# Example: small/10_timeout_failure
# File: TASKS.py
# Scenario: timeout failure

SPEC = module_spec(
  project_id="example_small_10",
  tasks=[
    task(
      "slow_timeout",
      timeout_s=1,
      steps=[cmd("sh", "-c", "sleep 2")]
    )
  ]
)
SPEC
