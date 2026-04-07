# Example: large/25_remote_direct_build_and_artifact_roundtrip
# File: services/api/TASKS.py
# Scenario: remote direct build and artifact roundtrip

REMOTE = Remote(
  pool="build",
  required_tags=["builder"],
  required_capabilities=["linux"],
  transport=DirectHttps(),
)

SPEC = module_spec(
  tasks=[
    task(
      "build_remote",
      deps=["//:prepare_context"],
      steps=[
        cmd(
          "sh",
          "-c",
          "mkdir -p out && echo artifact-from-remote-build > out/remote-build-artifact.txt && echo remote-build-ok > out/remote-build.log",
        )
      ],
      execution=RemoteOnly(REMOTE),
    ),
    task(
      "verify_artifact",
      deps=[":build_remote"],
      steps=[
        cmd(
          "sh",
          "-c",
          "grep -q artifact-from-remote-build out/remote-build-artifact.txt && echo verify-local-ok > out/local-verify.log",
        )
      ],
    ),
    task(
      "release",
      deps=[":verify_artifact"],
      steps=[
        cmd(
          "sh",
          "-c",
          "cat out/remote-build-artifact.txt out/local-verify.log > out/release-summary.txt",
        )
      ],
    ),
  ]
)
SPEC
