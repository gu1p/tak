# large/28_hybrid_local_remote_test_suite_failure_with_logs

## Why This Matters

Successful systems also need deterministic failure behavior. This example proves that a failing remote test phase still returns actionable logs and failure reason artifacts to local workspace consumers.

## Copy-Paste Starter

```python
REMOTE = Execution.Remote(
    pool="test",
    required_tags=["builder"],
    required_capabilities=["linux"],
    transport=Transport.DirectHttps(),
    runtime=Runtime.Image("alpine:3.20"),
)

SPEC = module_spec(
    tasks=[
        task(
            "unit_local",
            steps=[cmd("sh", "-c", "mkdir -p out && echo unit-local-pass > out/local-unit.log")],
        ),
        task(
            "remote_suite",
            deps=[":unit_local"],
            steps=[
                cmd(
                    "sh",
                    "-c",
                    "mkdir -p out && "
                    "echo test_auth_pass > out/remote-test-output.log && "
                    "echo test_payments_fail_expected_200_got_500 >> out/remote-test-output.log && "
                    "echo failure_reason_assertion_mismatch_in_payments_handler > out/remote-failure-reason.txt && "
                    "exit 3",
                )
            ],
            execution=REMOTE,
        ),
    ]
)
SPEC
```

## Parameter Alternatives

| Parameter | Current value | Alternatives | Behavior impact |
|---|---|---|---|
| remote task exit behavior | `exit 3` | `exit 0` or retried failure codes | Controls fail-fast behavior and whether pipeline ends as failure. |
| execution mode | `REMOTE` | `Execution.Policy(...)` | Allows dynamic fallback when remote is unavailable. |
| run strategy | default fail-fast | `tak run ... --keep-going` | Continue running independent targets after failures. |

## Runbook

Bootstrap a matching direct agent before running locally:

```bash
takd init --transport direct --base-url http://127.0.0.1:0 --pool test --tag builder --capability linux
takd serve
takd status
tak remote add "$(takd token show --wait)"
```

If the remote server does not come up cleanly, inspect it in place with `takd logs --lines 50`.

1. `tak list`
2. `tak explain //apps/web:remote_suite`
3. `tak graph //apps/web:remote_suite --format dot`
4. `tak run //apps/web:remote_suite`

## Expected Signals

- Command exits non-zero for `remote_suite`.
- Stderr reports task failure.
- Failure diagnostics remain in `out/` for inspection.

## Artifacts

- `out/local-bootstrap.log`
- `out/local-unit.log`
- `out/remote-test-output.log`
- `out/remote-failure-reason.txt`
