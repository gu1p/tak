# Example: small/08_retry_fixed_fail_once
# File: TASKS.py
# Scenario: retry fixed fail once

RETRY_SESSION = session(
  "retry-fixed",
  execution=Execution.Local(),
  reuse=SessionReuse.SharedWorkspace(max_parallel_tasks=1),
  affinity=Affinity.RequireSameNode("retry-fixed"),
)

SPEC = module_spec(
    spec_version=2,
  project_id="example_small_08",
  tasks=[
    task(
      "flaky_fixed",
      retry=retry(attempts=2, on_exit=[42], backoff=fixed(0)),
      outputs=[path("out/retry_fixed.txt")],
      steps=[
        cmd(
          "sh", "-c",
          "mkdir -p out && if [ -f out/seen_fixed ]; then echo recovered > out/retry_fixed.txt; exit 0; else touch out/seen_fixed; exit 42; fi"
        )
      ],
      use_session=RETRY_SESSION,
    )
  ]
)
SPEC
