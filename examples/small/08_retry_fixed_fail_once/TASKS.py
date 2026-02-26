# Example: small/08_retry_fixed_fail_once
# File: TASKS.py
# Scenario: retry fixed fail once

SPEC = module_spec(
  tasks=[
    task(
      "flaky_fixed",
      retry=retry(attempts=2, on_exit=[42], backoff=fixed(0)),
      steps=[
        cmd(
          "sh", "-c",
          "mkdir -p out && if [ -f out/seen_fixed ]; then echo recovered > out/retry_fixed.txt; exit 0; else touch out/seen_fixed; exit 42; fi"
        )
      ]
    )
  ]
)
SPEC
