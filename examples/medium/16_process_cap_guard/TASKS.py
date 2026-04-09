# Example: medium/16_process_cap_guard
# File: TASKS.py
# Scenario: process cap guard

SPEC = module_spec(
  project_id="example_medium_16",
  limiters=[process_cap("simulator", max_running=2, match="sim", scope=MACHINE)],
  tasks=[
    task(
      "process_guarded",
      needs=[need("simulator", 1, scope=MACHINE)],
      steps=[cmd("sh", "-c", "mkdir -p out && echo process-cap > out/process_cap.txt")]
    )
  ]
)
SPEC
