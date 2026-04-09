# Example: small/05_script_step_with_interpreter
# File: TASKS.py
# Scenario: script step with interpreter

SPEC = module_spec(
  project_id="example_small_05",
  tasks=[
    task(
      "script_gen",
      steps=[script("scripts/write_value.sh", "out/script.txt", interpreter="sh")]
    )
  ]
)
SPEC
