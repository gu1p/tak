# Example: large/23_contention_heavy_daemon_coordination
# File: TASKS.py
# Scenario: contention heavy daemon coordination

SPEC = module_spec(
  project_id="example_large_23",
  includes=[path("apps/a"), path("apps/b"), path("apps/c")],
  limiters=[lock("ui_lock", scope=MACHINE)],
  tasks=[
    task(
      "orchestrate",
      deps=["//apps/a:ui", "//apps/b:ui", "//apps/c:ui"],
      steps=[cmd("sh", "-c", "mkdir -p out && echo orchestrate >> out/contention.log")]
    )
  ]
)
SPEC
