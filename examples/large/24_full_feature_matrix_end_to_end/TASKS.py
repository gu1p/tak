# Example: large/24_full_feature_matrix_end_to_end
# File: TASKS.py
# Scenario: full feature matrix end to end

SPEC = module_spec(
  project_id="example_large_24",
  includes=[path("apps/qa"), path("libs/common")],
  limiters=[
    resource("cpu", 8, unit="slots", scope=MACHINE),
    resource("ram_gib", 32, unit="gib", scope=MACHINE),
    lock("ui_lock", scope=MACHINE),
    rate_limit("start_rl", burst=5, refill_per_second=10, scope=MACHINE),
    process_cap("simulator", max_running=2, match="sim", scope=MACHINE),
  ],
  queues=[
    queue_def("qa_fifo", slots=1, discipline=FIFO, scope=MACHINE),
    queue_def("qa_priority", slots=1, discipline=PRIORITY, scope=MACHINE),
  ],
  defaults={
    "retry": retry(attempts=2, on_exit=[44], backoff=fixed(0)),
    "tags": ["full-matrix"],
  },
  tasks=[
    task("bootstrap", steps=[cmd("sh", "-c", "mkdir -p out && echo bootstrap >> out/full_matrix.log")]),
    task(
      "seed_flaky",
      deps=[":bootstrap"],
      steps=[cmd("sh", "-c", "mkdir -p out && if [ -f out/full_seen ]; then echo seed-ok >> out/full_matrix.log; exit 0; else touch out/full_seen; exit 44; fi")]
    ),
  ]
)
SPEC
