#!/usr/bin/env bash
# Decisive A/B isolation test for the B->A onion-dial hang.
#
#   Run A (shared):     B's broker reuses the HS-service arti client (default).
#   Run B (own client): TAKD_BROKER_OWN_CLIENT=1 -> broker bootstraps its own arti
#                       client (separate arti/state/broker dir) for peer dials.
#
# One node A (remote) is shared by both runs. For each B variant we register A and
# watch whether node-a reaches `connected` and a tiny remote task runs. Full arti
# client-side tracing is on so we can SEE descriptor-fetch / rendezvous / circuit
# activity (or silence) during the dial.
#
# Report -> /tmp/ab_report.txt ; per-run B logs -> /tmp/ab_<variant>_b.log
set -uo pipefail
cd /media/gp/Projects1/gu1p/tak
REP=/tmp/ab_report.txt; : > "$REP"
say(){ printf '%s\n' "$*" >> "$REP"; }
# arti client-side tracing (Codex + my plan): hsclient/circmgr/netdir/dirmgr.
RL='info,takd=info,takd::daemon::protocol::broker::tor_client::connect=debug,takd::daemon::peer_manager::heartbeat=debug,tor_hsclient=debug,tor_circmgr=debug,tor_netdir=info,tor_dirmgr=warn,tor_hsservice=warn,tor_guardmgr=warn'

cp -f target/debug/tak docker/tor-test/tak
cp -f target/debug/takd docker/tor-test/takd
strip docker/tor-test/tak docker/tor-test/takd 2>/dev/null
docker build -t tak-tor-test:latest docker/tor-test >/tmp/ab_img.txt 2>&1
say "image_rc=$?"

docker rm -f takd-a >/dev/null 2>&1
docker run -d --name takd-a -e RUST_LOG="$RL" tak-tor-test:latest \
  sh -c 'takd init --node-id node-a --transport tor --pool default --tag smoke --capability mock && exec takd serve' >/dev/null 2>&1
A_TOKEN=$(docker exec takd-a takd token show --wait --timeout-secs 300 2>/dev/null | grep -m1 '^takd:')
say "A_onion_ready=$([ -n "$A_TOKEN" ] && echo YES || echo NO) token_len=${#A_TOKEN}"

run_variant() {
  local name="$1"; shift   # extra docker -e args
  say ""; say "########## VARIANT: $name ##########"
  docker rm -f takd-b >/dev/null 2>&1
  docker run -d --name takd-b -e RUST_LOG="$RL" -e TAKD_PEER_HEARTBEAT_TIMEOUT_MS=60000 "$@" tak-tor-test:latest \
    sh -c 'takd init --node-id node-b --transport tor && exec takd serve' >/dev/null 2>&1
  docker exec takd-b takd token show --wait --timeout-secs 300 >/dev/null 2>&1
  say "  B_onion_ready_rc=$?"
  docker exec takd-b tak remote add "$A_TOKEN" >>"$REP" 2>&1
  say "  remote_add_rc=$?"
  local conn=no
  for i in $(seq 1 18); do   # up to 4.5 min
    local st
    st=$(docker exec takd-b takd peers 2>/dev/null | grep -i node-a | tr -s ' ' | cut -d' ' -f1-3)
    say "    t=$((i*15))s peer: $st"
    echo "$st" | grep -qiw connected && { conn=yes; break; }
    sleep 15
  done
  say "  peer_connected=$conn"
  if [ "$conn" = yes ]; then
    docker exec takd-b sh -c 'rm -rf /work/probe && mkdir -p /work/probe'
    docker cp docker/tor-test/remote_probe_project/TASKS.py takd-b:/work/probe/TASKS.py >/dev/null 2>&1
    docker exec -w /work/probe takd-b tak run hello --remote >>"$REP" 2>&1
    say "  remote_run_rc=$?"
  fi
  docker cp takd-b:/root/.local/state/takd/service.log "/tmp/ab_${name}_b.log" >/dev/null 2>&1
  say "  --- B broker dial / heartbeat / hsclient lines ---"
  grep -aiE "onion dial|heartbeat ping|hsclient|rendezvous|descriptor|circmgr" "/tmp/ab_${name}_b.log" 2>/dev/null \
    | sed -E 's/^[0-9T:.Z-]+ +//' | cut -c1-150 | tail -25 >> "$REP"
}

run_variant shared
run_variant ownclient -e TAKD_BROKER_OWN_CLIENT=1

say ""; say "DONE"
