# Example: large/28_hybrid_local_remote_test_suite_failure_with_logs
# File: apps/web/TASKS.py
# Scenario: hybrid local + remote test suite (failure path with logs)

REMOTE = Remote(id="remote-hybrid-failure", endpoint="__TAK_REMOTE_ENDPOINT__")

SPEC = module_spec(
  tasks=[
    task(
      "unit_local",
      deps=["//:bootstrap_local"],
      steps=[cmd("sh", "-c", "mkdir -p out && echo unit-local-pass > out/local-unit.log")],
    ),
    task(
      "remote_suite",
      deps=[":unit_local"],
      steps=[
        cmd(
          "sh",
          "-c",
          "mkdir -p out && echo test_auth_pass > out/remote-test-output.log && echo test_payments_fail_expected_200_got_500 >> out/remote-test-output.log && echo failure_reason_assertion_mismatch_in_payments_handler > out/remote-failure-reason.txt && exit 3",
        )
      ],
      execution=RemoteOnly(REMOTE),
    ),
  ]
)
SPEC
