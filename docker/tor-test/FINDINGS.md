# Two-node real-Tor reproduction: findings

A reproduction harness that runs **two containers** talking over **real Tor**:

- **node A** (`takd-a`): `takd serve` only — a remote node. Publishes an onion HS.
- **node B** (`takd-b`): `tak` + local `takd serve` — the client/bridge. `tak` asks
  the local `takd` daemon to relay jobs to peers; the daemon's broker keeps warm
  Tor sessions and heartbeats them.

`MOCK_CONTAINER=true` on both so `takd` simulates container execution (no nested
Docker), letting us exercise the **real Tor transport** between the containers.

Run it: `bash docker/tor-test/verify_fix.sh` (build + two nodes + connect + remote
runs). Other scripts: `diag_nodeA.sh`, `two_node_test.sh`, `ab_dial_test.sh`. Host
binaries are copied into the image (`Dockerfile`) and git-ignored.

## Root cause (CONFIRMED end-to-end + by code review and a second-opinion review)

**The onion inbound path did not speak HTTP/2, but the broker prefers HTTP/2.**

- The TCP/direct server entry `handle_remote_v1_stream`
  (`crates/takd/src/daemon/remote/http_server.rs:33`) FIRST sniffs the protocol
  prefix (`prefixed_io.rs`, the `PRI * HTTP/2.0…` preface) and routes to a hyper
  HTTP/2 server OR the HTTP/1.1 reader.
- The **live onion** path (`crates/takd/src/service/tor/rend.rs`) called
  `handle_remote_v1_http_stream` **directly** — the HTTP/1.1-only reader — so the
  onion server never spoke HTTP/2.
- The broker prefers HTTP/2 first
  (`crates/takd/src/daemon/protocol/broker/tor_client/remote_exchange.rs:14-47`).
  So over the onion it sent the HTTP/2 preface; A's HTTP/1.1 reader failed at
  `read request bytes` (`http_server/request.rs:54`), logged
  `ERROR …service::tor::rend: takd onion service stream handling failed: read
  request bytes`, the ping never completed, and B's peer flapped to `unreachable`
  (`detail="ping timed out"`). This ALSO explains the earlier 2.87 MB POST
  `http2_request_failed` (POST can't fall back to HTTP/1.1, and the onion server
  couldn't do HTTP/2).

How we proved it was NOT the other suspects:
- **Shared vs own Tor client (refuted by A/B test).** We added a switch to make B's
  broker use its own arti client instead of reusing the hidden-service client. BOTH
  variants behaved identically: the onion dial *succeeded* (`broker onion dial
  succeeded`, full `tor_hsclient`/`tor_circmgr` activity) yet the ping still timed
  out. So the dial/transport layer was fine; the failure was above it, in the HTTP
  exchange. (All the own-client scaffolding was reverted afterward.)
- A's self-probe works because it uses an HTTP/1.1 hyper client
  (`service/tor/probe/http_client.rs`), which the HTTP/1.1-only reader accepts.

The sibling onion handler at `crates/takd/src/daemon/remote/tor_server.rs:63-67`
ALREADY routed through the prefix-sniffing `handle_remote_v1_stream` — so
`service/tor/rend.rs` was simply the divergent, buggy copy.

## The fix

`crates/takd/src/service/tor/rend.rs`: route accepted onion streams through
`handle_remote_v1_stream` (prefix-sniffing; supports HTTP/2 + HTTP/1.1), matching
the TCP path and the sibling onion handler. One import + one call. This fixes the
heartbeat (peer reaches `connected`) AND the large HTTP/2 POST submit over onion.

Verified: node-a reaches `takd peers` state `connected` in ~30s (was: hang →
`unreachable` forever); a 2.87 MB workspace submit is `accepted by node-a`.

## MOCK_CONTAINER (so takd can run inside a container with no nested runtime)

`tak_core::mock::mock_container_enabled()` reads env `MOCK_CONTAINER`
(truthy: 1/true/yes/on). Wired in:
- `tak-exec/src/engine/runtime_metadata.rs` — `should_use_simulated_container_runtime()`
  now also returns true under MOCK_CONTAINER, so the engine probe is skipped and the
  task runs on the host (no Docker/Podman). This reuses the existing, tested
  simulate path and covers BOTH the local `tak` path and the takd remote-worker
  path. (Without this, the remote worker failed: `no container engine available;
  attempted probes: docker`.)
- takd background touchpoints gated under the same flag so they don't spam a missing
  Docker: exec-root probe (`daemon/remote/runtime.rs` `skip_exec_root_probe`),
  cleanup janitor container reap (`cleanup_janitor.rs`), usage sampler
  (`tak_container_usage.rs`).

## Build/image facts

Host-built debug binaries need only glibc ≤ 2.39, libgcc, libm, liblzma, libbz2 —
NO OpenSSL (arti = rustls; sqlite bundled; protoc vendored). So `debian:trixie-slim`
(glibc 2.41) + host-built (stripped) binaries works; bookworm-slim (2.36) is too
old. `takd serve` flags are ONLY `--config-root`/`--state-root`. Get A's invite via
`takd token show --wait` — note it prints the token on STDOUT and a security WARNING
on STDERR, so capture stdout and take the `takd:`-prefixed line.

Budget ≥120s per node for onion readiness (arti consensus + onion publish + self
probe). Both nodes use `--transport tor`; B needs its own onion bootstrap to dial
out.

## Instrumentation changes (signal vs noise)

- `takd serve` logging now tees to **stderr** as well as `<state>/service.log`
  (`logging.rs`), so `docker logs`/terminal show lifecycle (it was file-only — the
  #1 reason this was historically hard to debug).
- Downgraded INFO→debug on two flood sites: `live_readiness_support.rs` per-poll
  onion-state line and `probe/health_detail.rs` per-15s self-probe line.
- Added broker onion-dial logging (start/succeeded/attempt-failed/deadline) in
  `connect.rs` and heartbeat ping outcome logging (timeout/failure/ok) in
  `heartbeat.rs`, so peer-state changes and dial outcomes are no longer silent.
