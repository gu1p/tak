# Example: medium/12_resource_cpu_ram_pool
# File: TASKS.py
# Scenario: resource cpu ram pool

SPEC = module_spec(
  limiters=[
    resource("cpu", 8, unit="slots", scope=MACHINE),
    resource("ram_gib", 32, unit="gib", scope=MACHINE),
  ],
  tasks=[
    task(
      "heavy",
      needs=[need("cpu", 2, scope=MACHINE), need("ram_gib", 4, scope=MACHINE)],
      steps=[cmd("sh", "-c", "mkdir -p out && echo heavy > out/resources.txt")]
    )
  ]
)
SPEC
