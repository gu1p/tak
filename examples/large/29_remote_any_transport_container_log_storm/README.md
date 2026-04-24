# large/29_remote_any_transport_container_log_storm

## Scenario Goal
Run a very noisy app inside a containerized remote runtime without pinning the transport. The same target can land on a matching direct node or a matching Tor node.

Large tier: transport-agnostic remote execution, containerized runtime metadata, and sustained stdout/stderr streaming.

## What This Example Exercises
- omitted `transport` in `Execution.Remote(...)`, which means any matching enabled remote transport
- `Runtime.Image("alpine:3.20")`
- heavy remote stdout and stderr streaming while the task is still running
- local verification of remote-produced artifacts after sync

## Copy-Paste Starter

```python
REMOTE = Execution.Remote(
    pool="build",
    required_tags=["builder"],
    required_capabilities=["linux"],
    runtime=Runtime.Image("alpine:3.20"),
)

SPEC = module_spec(tasks=[
    task(
        "container_log_storm",
        steps=[cmd("sh", "-c", "printf 'log-storm-stdout-001\n'")],
        execution=REMOTE,
    ),
])
SPEC
```

`Transport.Any()` also exists if you want to make that choice explicit, but the intended user path is to omit `transport` entirely.

## Runbook

From this example directory:

1. `tak list`
2. `tak explain //apps/logstorm:observe_container_log_storm`
3. `tak graph //apps/logstorm:observe_container_log_storm --format dot`
4. `tak run //apps/logstorm:observe_container_log_storm`

Direct node setup:

```bash
takd init --transport direct --base-url http://127.0.0.1:0 --pool build --tag builder --capability linux
takd serve
tak remote add "$(takd token show --wait)"
```

Tor node setup elsewhere:

```bash
takd init --transport tor --pool build --tag builder --capability linux
takd serve
tak remote add "$(takd token show --wait)"
```

If both a direct and a Tor node match, Tak uses the first reachable enabled remote in inventory order.

## Expected Artifacts
- `out/local-input.txt`
- `out/container-log-storm-summary.txt`
- `out/container-log-storm-verified.txt`
- `out/container-log-storm-report.txt`

## Notes
Manual runs require the matched remote host to have a working container engine. Catalog contract runs simulate only the container-engine probe so CI can stay deterministic.
