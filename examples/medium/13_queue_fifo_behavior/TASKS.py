# Example: medium/13_queue_fifo_behavior
# File: TASKS.py
# Scenario: queue fifo behavior

SPEC = module_spec(
    spec_version=2,
  project_id="example_medium_13",
  queues=[queue_def("ui_fifo", slots=1, discipline=QueueDiscipline.Fifo, scope=Scope.Machine)],
  tasks=[
    task(
      "queued_fifo",
      queue=queue_use("ui_fifo", scope=Scope.Machine, slots=1, priority=0),
      outputs=[path("out/queue_fifo.txt")],
      steps=[cmd("sh", "-c", "mkdir -p out && echo fifo > out/queue_fifo.txt")]
    )
  ]
)
SPEC
