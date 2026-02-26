# Example: large/23_contention_heavy_daemon_coordination
# File: apps/a/TASKS.py
# Scenario: contention heavy daemon coordination

SPEC = module_spec(
  tasks=[
    task(
      "ui",
      needs=[need("ui_lock", 1, scope=MACHINE)],
      steps=[cmd("sh", "-c", "echo app-a-ui >> out/contention.log")]
    )
  ]
)
SPEC
