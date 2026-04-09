# Example: small/09_retry_exp_jitter_match_exit
# File: TASKS.py
# Scenario: retry exp jitter match exit

SPEC = module_spec(
  project_id="example_small_09",
  tasks=[
    task(
      "flaky_jitter",
      retry=retry(attempts=2, on_exit=[17], backoff=exp_jitter(min_s=0, max_s=0.01)),
      steps=[
        cmd(
          "sh", "-c",
          "mkdir -p out && if [ -f out/seen_jitter ]; then echo recovered > out/retry_jitter.txt; exit 0; else touch out/seen_jitter; exit 17; fi"
        )
      ]
    )
  ]
)
SPEC
