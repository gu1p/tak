#!/usr/bin/env bash
# Diagnostic variant of proto_check.sh: captures BOTH node A's server-side
# per-stream h2/h1 split AND node B's broker-side cause markers (h2 handshake
# timeout vs error vs ok, send_request failures, fallback+pin decisions).
# Leaves containers running (KEEP) for further inspection.
set -uo pipefail
cd /media/gp/Projects1/gu1p/tak
REP=/tmp/proto_diag_report.txt; : > "$REP"
say(){ printf '%s\n' "$*" | tee -a "$REP"; }
# info baseline + broker & http_server at debug so our diagnostic lines show.
RL='info,takd=info,takd::daemon::protocol::broker=debug,takd::daemon::remote::http_server=debug,tor_hsservice=warn,tor_dirmgr=warn,tor_guardmgr=warn,tor_proto=warn'

cp -f target/debug/tak docker/tor-test/tak
cp -f target/debug/takd docker/tor-test/takd
strip docker/tor-test/tak docker/tor-test/takd 2>/dev/null
docker build -t tak-tor-test:latest docker/tor-test >/tmp/proto_img.txt 2>&1
say "image_rc=$?"

docker rm -f takd-a takd-b >/dev/null 2>&1
docker run -d --name takd-a -e RUST_LOG="$RL" tak-tor-test:latest \
  sh -c 'takd init --node-id node-a --transport tor --pool default --tag smoke --capability mock && exec takd serve' >/dev/null 2>&1
docker run -d --name takd-b -e RUST_LOG="$RL" tak-tor-test:latest \
  sh -c 'takd init --node-id node-b --transport tor && exec takd serve' >/dev/null 2>&1

A_TOKEN=$(docker exec takd-a takd token show --wait --timeout-secs 300 2>/dev/null | grep -m1 '^takd:')
say "A_onion_ready=$([ -n "$A_TOKEN" ] && echo YES || echo NO)"
docker exec takd-b takd token show --wait --timeout-secs 300 >/dev/null 2>&1
docker exec takd-b tak remote add "$A_TOKEN" >/dev/null 2>&1
say "remote_add_rc=$?"

CONN=no
for i in $(seq 1 24); do
  docker exec takd-b takd peers 2>/dev/null | grep -qiw connected && { CONN=yes; break; }
  sleep 15
done
say "peer_connected=$CONN  (after ~$((i*15))s)"

# Let several heartbeats elapse so we observe steady-state protocol selection.
say "observing heartbeats for 60s..."
sleep 60

# Run the real repo task over Tor (drives the big submit POST).
docker exec takd-b sh -c 'rm -rf /work/repo && mkdir -p /work/repo'
git archive HEAD | docker exec -i takd-b tar -x -C /work/repo
docker exec -w /work/repo takd-b tak run line-limits-check --remote >/tmp/proto_run.txt 2>&1
say "big_remote_run_rc=$?"
say "run_result: $(grep -m1 ': ok\|failed\|Error\|timed out' /tmp/proto_run.txt | cut -c1-120)"

docker cp takd-a:/root/.local/state/takd/service.log /tmp/proto_a.log 2>/dev/null
docker cp takd-b:/root/.local/state/takd/service.log /tmp/proto_b.log 2>/dev/null
say ""
say "=== node A per-stream protocol (server side) ==="
say "HTTP/2 streams served : $(grep -c 'serving remote v1 stream over HTTP/2' /tmp/proto_a.log)"
say "HTTP/1.1 streams served: $(grep -c 'serving remote v1 stream over HTTP/1.1' /tmp/proto_a.log)"
say "read-request-bytes errors: $(grep -c 'read request bytes' /tmp/proto_a.log)"
say ""
say "=== node B broker-side cause markers (case-insensitive) ==="
say "h2 handshake ok        : $(grep -ic 'h2 handshake ok' /tmp/proto_b.log)"
say "h2 handshake timed out : $(grep -ic 'h2 handshake timed out' /tmp/proto_b.log)"
say "h2 handshake failed    : $(grep -ic 'h2 handshake failed' /tmp/proto_b.log)"
say "h2 send_request failed : $(grep -ic 'h2 send_request failed' /tmp/proto_b.log)"
say "fallback->h1           : $(grep -ic 'falling back to HTTP/1.1' /tmp/proto_b.log)"
say ""
say "=== node B: handshake timing samples (dial_ms / handshake_ms) ==="
grep -iE 'h2 handshake (ok|timed out|failed)' /tmp/proto_b.log | tail -12 | tee -a "$REP"
say ""
say "=== node B: ordered cause trace (first 40 broker protocol lines) ==="
grep -iE 'h2 handshake|send_request failed|falling back' /tmp/proto_b.log | head -40 | tee -a "$REP"
say DONE
