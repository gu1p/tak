# Example: large/25_remote_direct_build_and_artifact_roundtrip
# File: TASKS.py
# Scenario: remote direct build and artifact roundtrip

SPEC = module_spec(
    spec_version=2,
  project_id="example_large_25",
  includes=[path("services/api")],
  tasks=[
    task(
      "prepare_context",
      outputs=[path("out/local-context.txt")],
      steps=[cmd("sh", "-c", "mkdir -p out && echo local-context-ready > out/local-context.txt")],
    )
  ]
)
SPEC
