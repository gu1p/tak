# Example: medium/15_rate_limit_start_tokens
# File: TASKS.py
# Scenario: rate limit start tokens

SPEC = module_spec(
    spec_version=2,
  project_id="example_medium_15",
  limiters=[rate_limit("start_rl", burst=2, refill_per_second=10, scope=Scope.Machine)],
  tasks=[
    task(
      "rate_limited",
      needs=[need("start_rl", 1, scope=Scope.Machine, hold=Hold.AtStart)],
      outputs=[path("out/rate_limit.txt")],
      steps=[cmd("sh", "-c", "mkdir -p out && echo rate > out/rate_limit.txt")]
    )
  ]
)
SPEC
