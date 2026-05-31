#!/usr/bin/env bash
# Decisive "can they actually communicate over Tor" test.
# Reuses the already-healthy node A; relaunches node B with a heartbeat timeout
# long enough to survive the first COLD onion dial (the 30s default fires before
# a cold hidden-service dial completes). Then registers A, waits for the peer to
# reach Connected, and runs a TINY remote task (isolates upload-size effects).
#
# Reads A's full token from /tmp/a_full_token.txt (capture with:
#   docker exec takd-a takd token show > /tmp/a_full_token.txt )
set -uo pipefail
cd /media/gp/Projects1/gu1p/tak
REP=/tmp/comm_report.txt; : > "$REP"
say(){ printf '%s\n' "$*" >> "$REP"; }

A_TOKEN=$(grep -m1 "^takd:" /tmp/a_full_token.txt | tr -d "\r\n")
say "A_token_len=${#A_TOKEN}"

docker rm -f takd-b >/dev/null 2>&1
docker run -d --name takd-b \
  -e RUST_LOG='info,takd=debug,tak_exec=info,tor_hsservice=warn' \
  -e TAKD_PEER_HEARTBEAT_TIMEOUT_MS=120000 \
  tak-tor-test:latest \
  sh -c 'takd init --node-id node-b --transport tor && exec takd serve' >/dev/null 2>&1
say "B_relaunched=$?"

docker exec takd-b takd token show --wait --timeout-secs 300 >/dev/null 2>&1
say "B_onion_ready_rc=$?"

docker exec takd-b tak remote add "$A_TOKEN" >>"$REP" 2>&1
say "remote_add_rc=$?"

docker exec takd-b sh -c 'rm -rf /work/probe && mkdir -p /work/probe'
docker cp docker/tor-test/remote_probe_project/TASKS.py takd-b:/work/probe/TASKS.py >/dev/null 2>&1

CONN=no; WAITED=0
for i in $(seq 1 30); do
  if docker exec takd-b takd peers 2>/dev/null | grep -qiw 'connected'; then CONN=yes; WAITED=$((i*10)); break; fi
  sleep 10
done
say "peer_connected=$CONN waited=${WAITED}s"
say "=== peers ==="
docker exec takd-b takd peers >>"$REP" 2>&1

docker exec -w /work/probe takd-b tak run hello --remote >/tmp/comm_run.txt 2>&1
say "remote_run_rc=$?"
say "=== tak run hello --remote output ==="
cat /tmp/comm_run.txt >>"$REP"

say "=== node A: did it accept/execute the submit? (marker/submit/worker lines) ==="
docker exec takd-a sh -c 'grep -iE "TAK_REMOTE_OK_MARKER|mock-container|simulating|submit|worker|/v1/node/(submit|info|ping)" /root/.local/state/takd/service.log 2>/dev/null | tail -25' >>"$REP" 2>&1
say "=== node B: broker/place/peer/http2 lines ==="
docker exec takd-b sh -c 'grep -iE "place|broker|http2|connect_failed|handshake|peer|node-a|unreachable|select" /root/.local/state/takd/service.log 2>/dev/null | tail -30' >>"$REP" 2>&1
say DONE
