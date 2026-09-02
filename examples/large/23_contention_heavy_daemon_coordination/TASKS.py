# Example: large/23_contention_heavy_daemon_coordination
# File: TASKS.py
# Scenario: contention heavy daemon coordination

SPEC = module_spec(
    spec_version=2,
  project_id="example_large_23",
  includes=[path("apps/a"), path("apps/b"), path("apps/c")],
  limiters=[lock("ui_lock", scope=Scope.Machine)],
  tasks=[
    task(
      "orchestrate",
      deps=["//apps/a:ui", "//apps/b:ui", "//apps/c:ui"],
      outputs=[path("out/contention.log")],
      steps=[cmd("sh", "-c", "cat out/app-a-ui.txt out/app-b-ui.txt out/app-c-ui.txt > out/contention.log && echo orchestrate >> out/contention.log")]
    )
  ]
)
SPEC
