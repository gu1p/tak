# Example: large/27_hybrid_local_remote_test_suite_success
# File: apps/web/TASKS.py
# Scenario: hybrid local + remote test suite (success path)

REMOTE = Remote(
  pool="test",
  required_tags=["builder"],
  required_capabilities=["linux"],
  transport=DirectHttps(),
)

SPEC = module_spec(
  tasks=[
    task(
      "unit_local",
      deps=["//:bootstrap_local"],
      steps=[cmd("sh", "-c", "mkdir -p out && echo unit-local-pass > out/local-unit.log")],
    ),
    task(
      "integration_remote",
      deps=[":unit_local"],
      steps=[
        cmd(
          "sh",
          "-c",
          "mkdir -p out && echo integration-remote-pass > out/remote-integration.log && echo junit-remote-all-pass > out/remote-junit.txt",
        )
      ],
      execution=RemoteOnly(REMOTE),
    ),
    task(
      "suite_success",
      deps=[":integration_remote"],
      steps=[
        cmd(
          "sh",
          "-c",
          "cat out/local-bootstrap.log out/local-unit.log out/remote-integration.log out/remote-junit.txt > out/hybrid-suite-summary.txt",
        )
      ],
    ),
  ]
)
SPEC
