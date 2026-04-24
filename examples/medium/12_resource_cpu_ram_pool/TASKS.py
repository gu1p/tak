# Example: medium/12_resource_cpu_ram_pool
# File: TASKS.py
# Scenario: resource cpu ram pool

SPEC = module_spec(
  project_id="example_medium_12",
  limiters=[
    resource("cpu", 8, unit="slots", scope=Scope.Machine),
    resource("ram_gib", 32, unit="gib", scope=Scope.Machine),
  ],
  tasks=[
    task(
      "heavy",
      needs=[need("cpu", 2, scope=Scope.Machine), need("ram_gib", 4, scope=Scope.Machine)],
      steps=[cmd("sh", "-c", "mkdir -p out && echo heavy > out/resources.txt")]
    )
  ]
)
SPEC
