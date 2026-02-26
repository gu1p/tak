# Example: small/01_hello_single_task
# File: TASKS.py
# Scenario: hello single task

SPEC = module_spec(
  tasks=[
    task(
      "hello",
      doc="Writes a hello output file.",
      steps=[cmd("sh", "-c", "mkdir -p out && echo hello > out/hello.txt")],
      tags=["small", "hello"]
    )
  ]
)
SPEC
