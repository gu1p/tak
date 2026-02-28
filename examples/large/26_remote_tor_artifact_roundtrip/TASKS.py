# Example: large/26_remote_tor_artifact_roundtrip
# File: TASKS.py
# Scenario: remote tor artifact roundtrip

REMOTE = Remote(
  id="remote-tor-artifacts",
  transport=RemoteTransportMode.TorOnionService(endpoint="__TAK_REMOTE_ENDPOINT__"),
)

SPEC = module_spec(
  tasks=[
    task(
      "collect_remote_report",
      steps=[
        cmd(
          "sh",
          "-c",
          "mkdir -p out && echo tor-remote-artifact > out/tor-remote-artifact.txt && echo tor-transport-ok > out/tor-remote.log",
        )
      ],
      execution=RemoteOnly(REMOTE),
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
