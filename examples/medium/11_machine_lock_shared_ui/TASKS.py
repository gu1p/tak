# Example: medium/11_machine_lock_shared_ui
# File: TASKS.py
# Scenario: machine lock shared ui

SPEC = module_spec(
  project_id="example_medium_11",
  limiters=[lock("ui_lock", scope=Scope.Machine)],
  tasks=[
    task(
      "ui_test",
      needs=[need("ui_lock", 1, scope=Scope.Machine)],
      steps=[cmd("sh", "-c", "mkdir -p out && echo ui-lock > out/ui_lock.txt")]
    )
  ]
)
SPEC
