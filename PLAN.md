# Remote Tor Failure: Explanation and Fix Plan

## What is actually failing

The failing path is not "the server stopped working". The failing path is:

1. `takd` starts a real onion service.
2. `takd token show --wait` returns a token.
3. `tak remote add` runs its own client-side onion probe.
4. That client-side probe times out.

You can see that flow here:

- `crates/tak/tests/remote_cli_live_tor_smoke.rs:18` waits for the token.
- `crates/tak/tests/remote_cli_live_tor_smoke.rs:19` calls `add_remote(...)`.
- `crates/tak/tests/support/live_tor_remote.rs:19` builds the `tak remote add` command.
- `crates/tak/src/cli/remote_probe.rs:96` returns the timeout error when the onion probe never becomes reachable from that fresh client.

So the right question is not "why did `takd` say ready too early?" The right question is "why does the client-side Tor path behave differently from the server-side Tor path after readiness?"

## Re-checked root causes

### Root cause 1: client-side Tor bootstrap uses ambient defaults instead of Tak-owned directories

The main bug is that Tak's client-side Tor code is using `TorClientConfig::default()` in several places:

- `crates/tak/src/cli/remote_probe.rs:48`
- `crates/tak/src/cli/remote_probe.rs:116`
- `crates/tak/src/cli/remote_status/fetch.rs:66`
- `crates/tak/src/cli/remote_status/fetch.rs:133`
- `crates/tak-exec/src/engine/transport.rs:181`

In plain English: those code paths let Arti decide where its state and cache live. Tak is not giving those clients a Tak-owned state directory. That means:

- `tak remote add`
- `tak remote status`
- remote execution in `tak-exec`

all depend on ambient machine state instead of a deterministic Tak client state.

That is the cleanest explanation for why the live Tor smoke can fail after `takd` is already ready: the server and the client are not bootstrapping Tor the same way.

This is especially clear when you compare it with `takd`, which does the opposite. `takd` explicitly builds Arti config from directories under its own state root:

- `crates/takd/src/service/tor.rs:27`
- `crates/takd/src/service/tor.rs:29`
- `crates/takd/src/service/tor.rs:30`
- `crates/takd/src/service/tor.rs:161`
- `crates/takd/src/service/tor.rs:164`
- `crates/takd/src/agent/paths.rs:19`
- `crates/takd/src/agent/paths.rs:23`
- `crates/takd/src/agent/paths.rs:27`

So today `takd` has explicit Tor state ownership, but `tak` and `tak-exec` do not.

### Root cause 2: the live Tor smoke harness isolates config, but not client state

The smoke-test helper only sets `XDG_CONFIG_HOME`:

- `crates/tak/tests/support/tor_smoke.rs:53`
- `crates/tak/tests/support/tor_smoke.rs:58`

It does not set `XDG_STATE_HOME` for the `tak` subprocess.

In plain English: the test creates a temporary config directory for the CLI, but it does not create a temporary client-state root for Tor. So even in the smoke test, the client-side Tor bootstrap is still free to lean on ambient defaults outside the temp test directory.

This is not the deepest bug. The deepest bug is still the client code using `TorClientConfig::default()`. But the harness makes the problem much harder to reason about because it does not isolate client state.

### Root cause 3: this is not `takd token show --wait` returning too early

This was the part that needed the most careful re-check.

`takd token show --wait` only waits for the token file:

- `crates/takd/src/agent.rs:113`

But the important detail is when that token file gets written.

For the real Tor path, `takd` writes the token only after its startup probe succeeds:

- `crates/takd/src/service/tor.rs:64`
- `crates/takd/src/service/tor.rs:66`
- `crates/takd/src/service/tor.rs:112`
- `crates/takd/src/service/tor.rs:115`
- `crates/takd/src/service/tor.rs:116`

And that token-writing function is exactly what persists the ready base URL and token:

- `crates/takd/src/agent.rs:126`
- `crates/takd/src/agent.rs:143`

There is already a live `takd` contract test that proves the intended meaning of readiness:

- `crates/takd/tests/service_tor_live_behavior.rs:22`
- `crates/takd/tests/service_tor_live_behavior.rs:25`
- `crates/takd/tests/support/live_tor_http/mod.rs:35`
- `crates/takd/tests/support/live_tor_http/mod.rs:36`
- `crates/takd/tests/support/live_tor_http/mod.rs:42`
- `crates/takd/tests/support/live_tor_http/mod.rs:53`
- `crates/takd/tests/support/live_tor_http/mod.rs:65`
- `crates/takd/tests/support/live_tor_http/mod.rs:73`

That test bootstraps a separate Arti client and waits until it can fetch `/v1/node/info` through the onion address after token readiness.

So the plain-English conclusion is:

- `takd` readiness is not the broken contract.
- The client-side Tor bootstrap path is the broken contract.

## Why this slipped through earlier

Most of the Tor fixture tests do not exercise a real onion reachability path. They inject local overrides:

- `crates/tak/tests/support/examples_tor_fixture.rs:40`
- `crates/tak/tests/support/examples_tor_fixture.rs:41`
- `crates/tak/tests/support/examples_tor_fixture.rs:42`
- `crates/tak/tests/support/examples_tor_fixture.rs:43`

That setup bypasses the real client-side Tor bootstrap by dialing a local TCP address instead of requiring a fresh Arti client to discover the onion descriptor. So those tests are useful, but they mask this exact problem.

## The clean fix

The clean fix is to make all Tak client-side Tor usage explicit and deterministic, the same way `takd` already is.

### 1. Add one shared Tak client Tor-config helper

Add a shared helper that resolves a Tak-owned client state root and builds an Arti config from explicit directories.

The best home is `tak-exec`, because both `tak` and `tak-exec` need it and `tak` already depends on `tak-exec`.

That helper should:

- resolve a Tak client state root from `XDG_STATE_HOME` or `HOME`
- keep the state under Tak-owned directories
- build explicit `arti/state` and `arti/cache` paths
- return an Arti config built with `TorClientConfigBuilder::from_directories(...)`

This should mirror the pattern already used by `takd` in:

- `crates/takd/src/service/tor.rs:161`
- `crates/takd/src/service/tor.rs:164`
- `crates/takd/src/agent/paths.rs:19`
- `crates/takd/src/agent/paths.rs:23`
- `crates/takd/src/agent/paths.rs:27`

### 2. Replace every client-side `TorClientConfig::default()` call

Update these bug sites to use that shared helper:

- `crates/tak/src/cli/remote_probe.rs:48`
- `crates/tak/src/cli/remote_probe.rs:116`
- `crates/tak/src/cli/remote_status/fetch.rs:66`
- `crates/tak/src/cli/remote_status/fetch.rs:133`
- `crates/tak-exec/src/engine/transport.rs:181`

This removes the ambient-state behavior from all client-side Tor entry points.

### 3. Isolate client state in the live Tor harness

Make the smoke harness pass a temp-local `XDG_STATE_HOME` to `tak` subprocesses, not just `XDG_CONFIG_HOME`.

The relevant places are:

- `crates/tak/tests/support/tor_smoke.rs:53`
- `crates/tak/tests/support/live_tor.rs:8`
- `crates/tak/tests/support/live_tor_remote.rs:19`
- `crates/tak/tests/remote_cli_live_tor_smoke.rs:22`

This does not replace the production fix above. It makes the live test deterministic and verifies the intended model: Tak client state should belong to the temp test root, not to the developer machine.

### 4. Follow TDD when implementing

Per repo rules, implementation should start with tests, not production edits.

Recommended order:

1. Add a contract test for the new shared client Tor-config helper.
2. Add a contract test that the smoke harness passes `XDG_STATE_HOME`.
3. Update the live Tor smoke to use the isolated state root.
4. Only then change the production Tor client call sites.

### 5. Keep the fix narrow

Do not do any of these:

- do not weaken `tak remote add` by skipping its probe
- do not redefine the meaning of `takd token show --wait`
- do not add more retries as the main fix

Those would treat the symptom. The real problem is inconsistent Tor client configuration between `takd` and the Tak clients.

## Summary

The most likely root cause is not a flaky connection and not premature server readiness. The real bug is that Tak's client-side Tor code uses ambient Arti defaults, while `takd` uses explicit Tak-owned directories. The smoke harness then makes that mismatch worse by isolating config but not state.

The clean fix is to give `tak`, `tak remote status`, and `tak-exec` the same explicit Tor state ownership model that `takd` already has.
