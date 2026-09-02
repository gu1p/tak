# Example: small/05_script_step_with_interpreter
# File: TASKS.py
# Scenario: script step with interpreter

SPEC = module_spec(
    spec_version=2,
  project_id="example_small_05",
  tasks=[
    task(
      "script_gen",
      outputs=[path("out/script.txt")],
      steps=[script("scripts/write_value.sh", "out/script.txt", interpreter="sh")]
    )
  ]
)
SPEC
