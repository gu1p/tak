# Example: small/06_defaults_inheritance
# File: TASKS.py
# Scenario: defaults inheritance

SPEC = module_spec(
  project_id="example_small_06",
  defaults=Defaults(
    retry=retry(attempts=2, on_exit=[9], backoff=fixed(0)),
    tags=["default-tag"],
  ),
  tasks=[
    task(
      "apply_defaults",
      steps=[cmd("sh", "-c", "mkdir -p out && echo defaults > out/defaults.txt")]
    )
  ]
)
SPEC
