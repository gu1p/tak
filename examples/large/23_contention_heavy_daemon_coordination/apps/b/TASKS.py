# Example: large/23_contention_heavy_daemon_coordination
# File: apps/b/TASKS.py
# Scenario: contention heavy daemon coordination

SPEC = module_spec(
  tasks=[
    task(
      "ui",
      needs=[need("ui_lock", 1, scope=MACHINE)],
      steps=[cmd("sh", "-c", "mkdir -p out && echo app-b-ui >> out/contention.log")]
    )
  ]
)
SPEC
