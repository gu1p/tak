# Real Tor Two-Role Relay Test

This harness reproduces the production relay path:

```text
tak -> local takd -> Tor onion service / HTTP/2 -> remote takd
```

It builds two role-specific images:

- `tak-tor-remote`: contains only `takd`, simulating a remote worker node.
- `tak-tor-local`: contains `tak` and `takd`, simulating a client machine with a local bridge daemon.

Both images set `MOCK_CONTAINER=true`. That only skips Docker/Podman container
runtime operations; it does not mock Tor, HTTP/2, the local daemon, peer warming,
request forwarding, task events, result polling, or output streaming.

## Run

```bash
bash docker/tor-test/e2e-two-role.sh
```

Useful overrides:

```bash
KEEP=1 bash docker/tor-test/e2e-two-role.sh
TAK_TOR_E2E_TASK=line-limits-check bash docker/tor-test/e2e-two-role.sh
TAK_TOR_E2E_RUST_LOG='info,takd=debug,takd::daemon::protocol::broker=debug' bash docker/tor-test/e2e-two-role.sh
```

The default task is `generated-artifact-ignore-check` because it is a real
repository task that can run inside the minimal remote image when container
execution is simulated.

## Workflow

The script:

1. Builds host debug binaries for `tak` and `takd`.
2. Builds the remote-only and local-client Docker images.
3. Starts the remote `takd` node with Tor transport.
4. Retrieves the remote node's `takd:` invite token and onion address.
5. Starts the local `takd` bridge with Tor transport.
6. Adds the remote invite to the local client configuration.
7. Waits for local `takd` to report the remote peer as connected.
8. Copies the current repository working tree into the local container.
9. Runs `tak run <task> --remote` from inside the local container.
10. Runs `tak exec --remote` as an explicit repository probe that checks key
    files made it to the remote worker.
11. Verifies logs show local placement/forwarding and remote submit/worker completion.

## Logs

Each run writes logs under `.tmp/tor-e2e/<timestamp>/`:

- `local-docker.log`: `docker logs` for the local client container.
- `remote-docker.log`: `docker logs` for the remote node container.
- `local-service.log`: local `takd` structured service log.
- `remote-service.log`: remote `takd` structured service log.
- `tak-run.log`: captured `tak run` output from the local container.
- `peers-latest.txt`: final `takd peers` output from the local container.
- `remote-token.txt` and `remote-onion.txt`: retrieved remote identity material.

Success means `tak-run.log` exits successfully and the service logs contain:

- local `takd`: `placing remote task through Tor peer`
- local `takd`: `forwarding workspace upload stream over Tor`
- local `takd`: `forwarding remote HTTP request over Tor`
- remote `takd`: `workspace upload stream committed`
- remote `takd`: `remote submit received`
- remote `takd`: `remote worker task finished`

## Limitations

`MOCK_CONTAINER=true` means task steps run directly in the remote image rather
than inside Docker/Podman. Use tasks whose commands exist in the minimal image, or
extend the image for a specific probe. The network path remains real.

For Tor-broker submissions the workspace is streamed to the chosen remote peer
before the small submit request is sent. Client progress is byte-based, while
local and remote `takd` logs identify the bridge forwarding and remote commit
points for the streamed upload.
