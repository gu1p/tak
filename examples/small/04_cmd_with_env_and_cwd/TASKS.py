# Example: small/04_cmd_with_env_and_cwd
# File: TASKS.py
# Scenario: cmd with env and cwd

SPEC = module_spec(
  project_id="example_small_04",
  tasks=[
    task(
      "env_cmd",
      steps=[
        cmd("mkdir", "-p", "out"),
        cmd(
          "sh", "-c", "echo \"$TAK_ENV_MARKER\" > marker.txt",
          cwd="out",
          env={"TAK_ENV_MARKER": "ENV_OK"}
        )
      ]
    )
  ]
)
SPEC
