#!/usr/bin/env bash
# Decisive diagnostic: rebuild image with instrumented takd, launch A+B fresh,
# register A on B, and capture WHY the warm Tor session does/doesn't establish.
# RUST_LOG is tuned to surface the newly-added broker onion-dial + heartbeat lines
# while muting arti bootstrap noise. Report -> /tmp/diag_comm_report.txt
set -uo pipefail
cd /media/gp/Projects1/gu1p/tak
REP=/tmp/diag_comm_report.txt; : > "$REP"
say(){ printf '%s\n' "$*" >> "$REP"; }
# info baseline + DEBUG on the two instrumented modules; arti muted.
RL='info,takd=info,takd::daemon::protocol::broker::tor_client::connect=debug,takd::daemon::peer_manager::heartbeat=debug,tor_hsservice=warn,tor_dirmgr=warn,tor_guardmgr=warn'

cp -f target/debug/tak docker/tor-test/tak
cp -f target/debug/takd docker/tor-test/takd
strip docker/tor-test/tak docker/tor-test/takd 2>/dev/null
docker build -t tak-tor-test:latest docker/tor-test >/tmp/diag_comm_img.txt 2>&1
say "image_rc=$?"

docker rm -f takd-a takd-b >/dev/null 2>&1
docker run -d --name takd-a -e RUST_LOG="$RL" tak-tor-test:latest \
  sh -c 'takd init --node-id node-a --transport tor --pool default --tag smoke --capability mock && exec takd serve' >/dev/null 2>&1
docker run -d --name takd-b -e RUST_LOG="$RL" -e TAKD_PEER_HEARTBEAT_TIMEOUT_MS=120000 tak-tor-test:latest \
  sh -c 'takd init --node-id node-b --transport tor && exec takd serve' >/dev/null 2>&1
say "nodes_launched"

A_TOKEN=$(docker exec takd-a takd token show --wait --timeout-secs 300 2>/dev/null | grep -m1 '^takd:')
say "A_onion_ready=$([ -n "$A_TOKEN" ] && echo YES || echo NO)  A_token_len=${#A_TOKEN}"
docker exec takd-b takd token show --wait --timeout-secs 300 >/dev/null 2>&1
say "B_onion_ready_rc=$?"

docker exec takd-b tak remote add "$A_TOKEN" >>"$REP" 2>&1
say "remote_add_rc=$?"

# Poll peers up to 6 min; record state each 15s.
CONN=no
for i in $(seq 1 24); do
  st=$(docker exec takd-b takd peers 2>/dev/null | grep -i node-a | tr -s ' ' | cut -d' ' -f1-3)
  say "  t=$((i*15))s peer: $st"
  echo "$st" | grep -qiw connected && { CONN=yes; break; }
  sleep 15
done
say "peer_connected=$CONN"

if [ "$CONN" = yes ]; then
  docker exec takd-b sh -c 'rm -rf /work/probe && mkdir -p /work/probe'
  docker cp docker/tor-test/remote_probe_project/TASKS.py takd-b:/work/probe/TASKS.py >/dev/null 2>&1
  docker exec -w /work/probe takd-b tak run hello --remote >>"$REP" 2>&1
  say "remote_run_rc=$?"
fi

say "=== B: broker onion-dial + heartbeat lines (the previously-silent failure) ==="
docker exec takd-b sh -c 'grep -iE "onion dial|heartbeat ping|broker" /root/.local/state/takd/service.log 2>/dev/null | tail -25' >>"$REP" 2>&1
say "=== A: incoming /v1 + worker/submit lines ==="
docker exec takd-a sh -c 'grep -iE "/v1/node|worker|submit|mock-container|simulating|TAK_REMOTE_OK" /root/.local/state/takd/service.log 2>/dev/null | grep -ivE "self-probe|health_detail|live_readiness" | tail -15' >>"$REP" 2>&1
say DONE
