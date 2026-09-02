# Worker v2 HTTP/2 over Tor

Tak clients never dial workers. The local `takd` owns direct and Tor inventory,
placement, credentials, and worker communication. Daemon-to-worker traffic uses
the protocol-v2 HTTP surface rooted at `/v2/worker`.

The Tor harness in this directory validates that a daemon can:

- discover a protocol-v2 worker over an onion service;
- reuse an HTTP/2 session for heartbeat and execution traffic;
- dispatch a fenced attempt through `/v2/worker/attempts`;
- observe that attempt and download its declared artifacts; and
- reject a protocol mismatch with instructions to upgrade `tak`, `takd`, and
  workers together.

Use `proto_check.sh` for transport diagnostics and `verify_fix.sh` for the
two-node exercise. Read the initiating daemon's broker logs when classifying
HTTP/2 reuse or fallback; the receiving worker logs one line per connection, not
one line per request.

HTTP/1.1 remains a transport fallback for protocol-v2 requests. It is not a
protocol compatibility adapter: legacy execution requests are rejected and are
never guessed or resumed.
