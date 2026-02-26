# Example: large/23_contention_heavy_daemon_coordination
# File: TASKS.py
# Scenario: contention heavy daemon coordination

SPEC = module_spec(
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
