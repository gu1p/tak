# Example: medium/13_queue_fifo_behavior
# File: TASKS.py
# Scenario: queue fifo behavior

SPEC = module_spec(
  project_id="example_medium_13",
  queues=[queue_def("ui_fifo", slots=1, discipline=QueueDiscipline.Fifo, scope=Scope.Machine)],
  tasks=[
    task(
      "queued_fifo",
      needs=[need("cpu", 1, scope=Scope.Machine)],
      queue=queue_use("ui_fifo", scope=Scope.Machine, slots=1, priority=0),
      steps=[cmd("sh", "-c", "mkdir -p out && echo fifo > out/queue_fifo.txt")]
    )
  ]
)
SPEC
