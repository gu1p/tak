# Example: large/23_contention_heavy_daemon_coordination
# File: apps/c/TASKS.py
# Scenario: contention heavy daemon coordination

SPEC = module_spec(
    spec_version=2,
  tasks=[
    task(
      "ui",
      needs=[need("ui_lock", 1, scope=Scope.Machine)],
      outputs=[path("//out/app-c-ui.txt")],
      steps=[cmd("sh", "-c", "mkdir -p out && echo app-c-ui > out/app-c-ui.txt", cwd="//")]
    )
  ]
)
SPEC
