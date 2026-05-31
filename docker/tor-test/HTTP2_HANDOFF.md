# Handoff prompt: make HTTP/2 the reliable transport for takd over Tor

You are picking up work on **tak/takd** (`/media/gp/Projects1/gu1p/tak`), a remote
task orchestrator that ships jobs between nodes over Tor v3 onion services using the
`arti-client` crate. `tak` (client) relays jobs through a local `takd serve`
daemon's **broker**, which keeps warm Tor sessions to remote `takd` nodes and speaks
a remote-v1 HTTP API (`/v1/node/ping`, `/v1/node/info`, submit, events, result).

**Goal:** HTTP/2 should be the *primary, reliable* transport between nodes over the
onion — not a best-effort attempt that silently degrades to HTTP/1.1. Right now h2
works but is used on a minority of streams. Find out why, and make h2 the dependable
default while keeping h1 only as a genuine last resort.

---

## Where this started (the original bug — already FIXED, context only)

Two `takd` nodes on `--transport tor` could not talk: `takd peers` showed the remote
peer permanently `unreachable` / `ping timed out`, and large job submits failed with
`http2_request_failed`. The historical symptom was "all Tor peers are unreachable."

Root cause (fixed this session): the **live onion inbound path bypassed HTTP/2
entirely.** The TCP/direct server entry `handle_remote_v1_stream`
(`crates/takd/src/daemon/remote/http_server.rs:33`) sniffs the protocol prefix
(`prefixed_io.rs`, the `PRI * HTTP/2.0…` preface) and serves **either** a hyper h2
server **or** the HTTP/1.1 reader. But the live onion handler
`crates/takd/src/service/tor/rend.rs` called `handle_remote_v1_http_stream`
**directly** — the HTTP/1.1-only reader — so the onion server never spoke h2. Since
the broker prefers h2 first, it sent the h2 preface, the h1 reader failed at
`read request bytes` (`http_server/request.rs:54`), the ping never completed, and
the peer flapped to `unreachable`.

**The fix (in the working tree):** `service/tor/rend.rs` now routes accepted onion
streams through the prefix-sniffing `handle_remote_v1_stream` (supports h2 + h1),
matching the TCP path and the sibling onion handler at
`crates/takd/src/daemon/remote/tor_server.rs:63-67`. Verified end-to-end: peer
reaches `connected` in ~30s and a 2.87 MB workspace submit is accepted by the remote
node over Tor.

(Also done this session, see `FINDINGS.md`: a `MOCK_CONTAINER` env mode so `takd`
simulates container execution inside a container with no nested Docker; `takd serve`
logging now tees to stderr as well as `<state>/service.log`; two noisy INFO sites
downgraded; broker-dial + heartbeat-ping outcomes are now logged.)

## What we measured after the fix (why this handoff exists)

We added per-stream server-side logging (which branch of the prefix sniff handled
the stream — `http_server.rs:43` h2 vs `:47` h1, now at `debug`) and ran the
two-container test again. Node A (the remote node) reported, across one session
(heartbeat pings + one big submit):

```
HTTP/2 streams served : 1
HTTP/1.1 streams served: 4
read-request-bytes errors: 0     ← the original bug is gone
```

And on one run the 2.87 MB submit **timed out** (`infra error: local takd daemon
timed out while contacting remote node … for <phase>`), while on an earlier run it
succeeded. So: **h2 over onion is functional but used on a minority of streams, the
peer keeps getting pinned to h1, and large transfers are flaky.**

---

## The problems to investigate (ranked, with file:line)

### 1. The HTTP/2 handshake timeout is onion-blind and far too short — PRIME SUSPECT
`crates/takd/src/daemon/protocol/broker/tor_client/http2_session.rs:8`
```rust
const HTTP2_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);
```
Used at `http2_session.rs:25` wrapping `hyper::client::conn::http2::handshake(...)`.
A **cold onion circuit** has multi-second round trips; a 2s budget for the h2
SETTINGS handshake will usually miss → `http2_unavailable`.

Note the asymmetry: tak-exec already raises *its* remote phase timeouts to a Tor
floor — `phase_timeout(target, requested) = requested.max(min_phase_timeout(target))`
(`crates/tak-exec/src/engine/transport.rs:56`), and for Tor `min_phase_timeout`
returns `tor_connect_timeout()` (`transport.rs:86-90`; env-tunable via
`TAK_TOR_PROBE_TIMEOUT_MS`, default 120s). So the *outer* phases already tolerate
slow onion links — but this broker-internal h2 handshake timeout is a flat **2s**
with **no onion awareness**, so h2 dies long before the outer budget. Make it
onion-aware (e.g. 15–30s for `.onion` peers, or derive it from
`tor_connect_timeout()` in `tor_client/config.rs`), ideally env-configurable like
the other Tor timeouts there.

### 2. One h2 failure pins the peer to HTTP/1.1 for *all* future requests
`crates/takd/src/daemon/protocol/broker/tor_client/remote_exchange.rs`:
- `prefer_http2 = broker.remote_protocol(...) != Some(RemoteProtocol::Http1)` (~line 14).
- On h2 success → `set_remote_protocol(Http2)` (~line 34); on h2 failure that falls
  through to h1 → `set_remote_protocol(Http1)` (~line 61-68, the
  "remember as HTTP/1.1-only" branch).

Protocol memory lives in `tor_client/protocol_memory.rs` (`RemoteProtocol::{Http2,
Http1}`, `remote_protocol` / `set_remote_protocol`; key = `endpoint\nnode_id`).
**Consequence:** the very first GET heartbeat, if its h2 handshake misses the 2s
budget (problem #1), pins the peer to h1 — so every later request, **including POST
submits**, skips h2 entirely. Question for you: should one transient h2 handshake
miss durably pin the peer? Consider making the pin time-boxed (retry h2
periodically), or not pinning on `http2_unavailable` (timeout-class) failures at all.

### 3. POST cannot fall back, by design — interacts badly with #1+#2
`remote_exchange.rs:94` `can_fallback_method` allows h1 fallback for
`http2_request_failed`/`connect_failed` only on GET/HEAD/OPTIONS/PUT/DELETE — **not
POST** (intentional: replaying a partially-processed POST could duplicate side
effects). Correct in isolation, but combined with #1+#2: the GET heartbeat fails h2
→ pins h1 → the POST submit then runs over h1 anyway. If you fix #1/#2 so h2 is
reliable, keep this as the safety net. If instead you want POSTs to fall back,
first verify remote submit idempotency (`submit_key` /
`sanitize_submit_idempotency_key`, `abandon_unfinished_submits` under
`crates/takd/src/daemon/remote/`) before relaxing it.

### 4. Large h2 POST over a single arti DataStream — does it actually complete?
The big-submit timeout (`tak-exec` phase timeout: base `REMOTE_RESULT_TIMEOUT = 10s`
at `protocol_result_http.rs:18`, floored to `tor_connect_timeout()` over Tor via
`phase_timeout` → `min_phase_timeout`) needs confirming: is the 2.87 MB POST timing
out because it went over **h1** (no multiplexing, slow), or is there an **h2
flow-control / window** issue carrying a multi-MB body over one arti `DataStream`?
Drive a submit *known* to be on h2 and watch `h2=trace,hyper=trace` on both ends.
Check whether the h2 server (`crates/takd/src/daemon/remote/http_server/http2.rs`,
`handle_remote_v1_http2_stream`) needs initial-window / max-frame tuning for big
bodies over high-latency onion links.

### 5. Per-request onion dials vs. session reuse
The broker pools h2 sessions (`tor_client.rs` `http2_session` + `session_pool.rs`)
but legacy h1 uses `Connection: close` (`remote_exchange.rs`
`legacy_http_request_bytes`), i.e. a fresh onion stream per request. Confirm whether
warm h2 session reuse actually happens across heartbeats (it should make
steady-state pings <1s) or whether sessions are evicted/rebuilt each cycle. Pool key
is `endpoint\nnode_id\nauth` (`session_pool.rs` `evict_http2_session_for_peer`).

---

## Files to look at

- `crates/takd/src/daemon/protocol/broker/tor_client/http2_session.rs` — handshake timeout (#1).
- `crates/takd/src/daemon/protocol/broker/tor_client/remote_exchange.rs` — prefer/fallback + pinning (#2, #3).
- `crates/takd/src/daemon/protocol/broker/tor_client/protocol_memory.rs` — the Http1/Http2 memory (#2).
- `crates/takd/src/daemon/protocol/broker/tor_client/config.rs` — existing Tor timeout env knobs to mirror for a configurable handshake timeout.
- `crates/takd/src/daemon/remote/http_server/http2.rs` + `prefixed_io.rs` — server-side h2 path (#4).
- `crates/tak-exec/src/engine/transport.rs` (`min_phase_timeout`, `phase_timeout`) + `protocol_result_http*` — outer phase timeouts (#4).

## How to reproduce / measure (harness is built and ready)

Everything is in `docker/tor-test/` (full writeup in `FINDINGS.md`):
- `bash docker/tor-test/verify_fix.sh` — build image, run node A + node B over real Tor, connect, run remote tasks.
- `bash docker/tor-test/proto_check.sh` — same, but prints node A's per-stream
  **HTTP/2 vs HTTP/1.1** counts. This is the metric to drive toward "all h2".
- Per-stream protocol log lines are at `http_server.rs:43/47` (`debug`); set
  `RUST_LOG=…,takd::daemon::remote::http_server=debug` while iterating.
- `MOCK_CONTAINER=true` is set in the image so `takd` simulates container execution
  (no nested Docker); the Tor transport is real.
- Gotcha: `takd token show` prints the token on **stdout** and a security WARNING on
  **stderr** — capture `2>/dev/null` and take the `takd:`-prefixed line.
- Logs: `takd serve` tees tracing to stderr AND `<state>/service.log`. Read node A:
  `docker cp takd-a:/root/.local/state/takd/service.log -`.

## Definition of done

- A clean two-node run where node A reports **HTTP/2 streams ≥ N, HTTP/1.1 streams =
  0** across heartbeats *and* the big submit (via `proto_check.sh`).
- The 2.87 MB submit completes over h2 reliably across several runs (no phase timeouts).
- h1 remains reachable only as a real fallback (a genuinely h2-incapable peer), not
  as the default after a transient handshake miss.
- New/adjusted timeouts are onion-aware and ideally env-configurable; add a
  regression test for the protocol-selection logic.

## Caveat / confidence

The #1 (2s handshake) → #2 (h1 pinning) chain is a strong, evidence-consistent
hypothesis (the 2s constant, the 1-vs-4 h2/h1 split, and the pin-on-fallback logic
all line up), but it is **not yet proven by a log line** — the h2 handshake timeout
itself isn't logged. First add a log at `http2_session.rs:25-28` distinguishing
"h2 handshake timed out" from other handshake errors, reproduce with
`proto_check.sh`, and confirm the timeout is what's firing before changing
constants. Don't assume; measure, then fix, then re-measure the h2/h1 split.
