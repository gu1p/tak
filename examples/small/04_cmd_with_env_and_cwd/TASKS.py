# Example: small/04_cmd_with_env_and_cwd
# File: TASKS.py
# Scenario: cmd with env and cwd

SPEC = module_spec(
  tasks=[
    task(
      "env_cmd",
      steps=[
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
