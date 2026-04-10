# Example: large/29_remote_any_transport_container_log_storm
# File: TASKS.py
# Scenario: transport-agnostic remote container log storm

SPEC = module_spec(
  project_id="example_large_29",
  includes=[path("apps/logstorm")],
  tasks=[
    task(
      "prepare_local_input",
      steps=[
        cmd(
          "sh",
          "-c",
          "mkdir -p out && echo local-log-storm-context-ready > out/local-input.txt",
        )
      ],
    ),
  ]
)
SPEC
