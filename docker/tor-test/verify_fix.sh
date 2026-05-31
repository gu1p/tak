#!/usr/bin/env bash
# Verify the http2-over-onion fix: rebuild image, launch A+B fresh, register A on
# B, wait for node-a to reach `connected`, then run a tiny remote task over Tor.
# Report -> /tmp/verify_report.txt
set -uo pipefail
cd /media/gp/Projects1/gu1p/tak
REP=/tmp/verify_report.txt; : > "$REP"
say(){ printf '%s\n' "$*" >> "$REP"; }
RL='info,takd=info,tak_exec=info,tor_hsservice=warn,tor_dirmgr=warn,tor_guardmgr=warn'

cp -f target/debug/tak docker/tor-test/tak
cp -f target/debug/takd docker/tor-test/takd
strip docker/tor-test/tak docker/tor-test/takd 2>/dev/null
docker build -t tak-tor-test:latest docker/tor-test >/tmp/verify_img.txt 2>&1
say "image_rc=$?"

docker rm -f takd-a takd-b >/dev/null 2>&1
docker run -d --name takd-a -e RUST_LOG="$RL" tak-tor-test:latest \
  sh -c 'takd init --node-id node-a --transport tor --pool default --tag smoke --capability mock && exec takd serve' >/dev/null 2>&1
docker run -d --name takd-b -e RUST_LOG="$RL" tak-tor-test:latest \
  sh -c 'takd init --node-id node-b --transport tor && exec takd serve' >/dev/null 2>&1
say "nodes_launched"

A_TOKEN=$(docker exec takd-a takd token show --wait --timeout-secs 300 2>/dev/null | grep -m1 '^takd:')
say "A_onion_ready=$([ -n "$A_TOKEN" ] && echo YES || echo NO)"
docker exec takd-b takd token show --wait --timeout-secs 300 >/dev/null 2>&1
say "B_onion_ready_rc=$?"

docker exec takd-b tak remote add "$A_TOKEN" >>"$REP" 2>&1
say "remote_add_rc=$?"

CONN=no
for i in $(seq 1 24); do
  st=$(docker exec takd-b takd peers 2>/dev/null | grep -i node-a | tr -s ' ' | cut -d' ' -f1-4)
  say "  t=$((i*15))s peer: $st"
  echo "$st" | grep -qiw connected && { CONN=yes; break; }
  sleep 15
done
say "peer_connected=$CONN"

if [ "$CONN" = yes ]; then
  docker exec takd-b sh -c 'rm -rf /work/probe && mkdir -p /work/probe'
  docker cp docker/tor-test/remote_probe_project/TASKS.py takd-b:/work/probe/TASKS.py >/dev/null 2>&1
  say "=== tak run hello --remote (over Tor; MOCK_CONTAINER simulates the container) ==="
  docker exec -w /work/probe takd-b tak run hello --remote >>"$REP" 2>&1
  say "remote_run_rc=$?"
  say "=== full repo (2.87MB) submit over onion: tak run line-limits-check --remote ==="
  docker exec takd-b sh -c 'rm -rf /work/repo && mkdir -p /work/repo'
  git archive HEAD | docker exec -i takd-b tar -x -C /work/repo
  docker exec -w /work/repo takd-b tak run line-limits-check --remote >>"$REP" 2>&1
  say "big_remote_run_rc=$?"
fi

say "=== node A: did it accept/execute submits? (worker/mock/marker) ==="
docker exec takd-a sh -c 'grep -aiE "worker|mock-container|simulating|TAK_REMOTE_OK_MARKER|read request bytes" /root/.local/state/takd/service.log 2>/dev/null | tail -15' >>"$REP" 2>&1
say DONE
