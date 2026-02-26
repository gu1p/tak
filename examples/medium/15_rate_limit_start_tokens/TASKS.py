# Example: medium/15_rate_limit_start_tokens
# File: TASKS.py
# Scenario: rate limit start tokens

SPEC = module_spec(
  limiters=[rate_limit("start_rl", burst=2, refill_per_second=10, scope=MACHINE)],
  tasks=[
    task(
      "rate_limited",
      needs=[need("start_rl", 1, scope=MACHINE, hold=AT_START)],
      steps=[cmd("sh", "-c", "mkdir -p out && echo rate > out/rate_limit.txt")]
    )
  ]
)
SPEC
