#!/usr/bin/env bash
# Definitively answer "is HTTP/2 carrying onion traffic?" by reading node A's
# server-side per-stream protocol log ("serving remote v1 stream over HTTP/2"
# vs "HTTP/1.1"). Rebuilds image, runs A+B, connects, runs the big repo task.
set -uo pipefail
cd /media/gp/Projects1/gu1p/tak
REP=/tmp/proto_report.txt; : > "$REP"
say(){ printf '%s\n' "$*" >> "$REP"; }
RL='info,takd=info,tor_hsservice=warn,tor_dirmgr=warn,tor_guardmgr=warn'

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

# Run the real repo task over Tor.
docker exec takd-b sh -c 'rm -rf /work/repo && mkdir -p /work/repo'
git archive HEAD | docker exec -i takd-b tar -x -C /work/repo
docker exec -w /work/repo takd-b tak run line-limits-check --remote >/tmp/proto_run.txt 2>&1
say "big_remote_run_rc=$?"
say "run_result: $(grep -m1 ': ok\|failed\|Error' /tmp/proto_run.txt | cut -c1-100)"

# THE ANSWER: per-stream protocol counts on node A (heartbeat pings + the submit).
docker cp takd-a:/root/.local/state/takd/service.log /tmp/proto_a.log 2>/dev/null
say ""
say "=== node A per-stream protocol (server side) ==="
say "HTTP/2 streams served : $(grep -c 'serving remote v1 stream over HTTP/2' /tmp/proto_a.log)"
say "HTTP/1.1 streams served: $(grep -c 'serving remote v1 stream over HTTP/1.1' /tmp/proto_a.log)"
say "read-request-bytes errors: $(grep -c 'read request bytes' /tmp/proto_a.log)"
say DONE
