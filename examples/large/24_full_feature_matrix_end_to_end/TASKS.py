# Example: large/24_full_feature_matrix_end_to_end
# File: TASKS.py
# Scenario: full feature matrix end to end

RETRY_SESSION = session(
  "full-matrix-seed",
  execution=Execution.Local(),
  reuse=SessionReuse.SharedWorkspace(max_parallel_tasks=1),
  affinity=Affinity.RequireSameNode("full-matrix-seed"),
)

SPEC = module_spec(
    spec_version=2,
  project_id="example_large_24",
  includes=[path("apps/qa"), path("libs/common")],
  limiters=[
    resource("cpu", 8, unit="slots", scope=Scope.Machine),
    resource("ram_gib", 32, unit="gib", scope=Scope.Machine),
    lock("ui_lock", scope=Scope.Machine),
    rate_limit("start_rl", burst=5, refill_per_second=10, scope=Scope.Machine),
    process_cap("simulator", max_running=2, match="sim", scope=Scope.Machine),
    lock("project_gate", scope=Scope.Project),
    lock("user_gate", scope=Scope.User),
    lock("worktree_gate", scope=Scope.Worktree),
  ],
  queues=[
    queue_def("qa_fifo", slots=1, discipline=QueueDiscipline.Fifo, scope=Scope.Machine),
    queue_def("qa_priority", slots=1, discipline=QueueDiscipline.Priority, scope=Scope.Machine),
  ],
  defaults=Defaults(
    retry=retry(attempts=2, on_exit=[44], backoff=fixed(0)),
    tags=["full-matrix"],
  ),
  tasks=[
    task(
      "bootstrap",
      outputs=[path("out/full-bootstrap.txt")],
      steps=[cmd("sh", "-c", "mkdir -p out && echo bootstrap > out/full-bootstrap.txt")],
    ),
    task(
      "seed_flaky",
      deps=[":bootstrap"],
      outputs=[path("out/full-seed.txt")],
      steps=[cmd("sh", "-c", "mkdir -p .retry out && if [ -f .retry/full-seen ]; then echo seed-ok > out/full-seed.txt; exit 0; else touch .retry/full-seen; exit 44; fi")],
      use_session=RETRY_SESSION,
    ),
  ]
)
SPEC
