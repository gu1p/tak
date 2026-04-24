# Example: medium/14_queue_priority_behavior
# File: TASKS.py
# Scenario: queue priority behavior

SPEC = module_spec(
  project_id="example_medium_14",
  queues=[queue_def("ui_priority", slots=1, discipline=QueueDiscipline.Priority, scope=Scope.Machine)],
  tasks=[
    task(
      "queued_priority",
      needs=[need("cpu", 1, scope=Scope.Machine)],
      queue=queue_use("ui_priority", scope=Scope.Machine, slots=1, priority=100),
      steps=[cmd("sh", "-c", "mkdir -p out && echo priority > out/queue_priority.txt")]
    )
  ]
)
SPEC
