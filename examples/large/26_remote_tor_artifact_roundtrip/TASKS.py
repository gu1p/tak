# Example: large/26_remote_tor_artifact_roundtrip
# File: TASKS.py
# Scenario: remote tor artifact roundtrip

REMOTE = Execution.Remote(
  pool="build",
  required_tags=["builder"],
  required_capabilities=["linux"],
  transport=Transport.TorOnionService(),
  runtime=Runtime.Image("alpine:3.20"),
)

SPEC = module_spec(
  project_id="example_large_26",
  tasks=[
    task(
      "collect_remote_report",
      outputs=[path("out")],
      steps=[
        cmd(
          "sh",
          "-c",
          "mkdir -p out && echo tor-remote-artifact > out/tor-remote-artifact.txt && echo tor-transport-ok > out/tor-remote.log",
        )
      ],
      execution=REMOTE,
    ),
    task(
      "consume_remote_report",
      deps=[":collect_remote_report"],
      steps=[
        cmd(
          "sh",
          "-c",
          "grep -q tor-remote-artifact out/tor-remote-artifact.txt && echo tor-roundtrip-local-ok > out/tor-roundtrip.txt",
        )
      ],
    ),
  ]
)
SPEC
