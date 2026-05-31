#!/usr/bin/env bash
# Focused single-node diagnostic: rebuild image, launch node A (takd-only, real
# Tor), wait for the onion to publish, and capture logs + network/thread state.
# Writes a compact summary to /tmp/nodeA_sum.txt and full logs to /tmp/nodeA.log.
set -uo pipefail
cd /media/gp/Projects1/gu1p/tak
OUT=/tmp/nodeA_sum.txt; : > "$OUT"
say(){ echo "$*" >> "$OUT"; }

cp -f target/debug/tak  docker/tor-test/tak
cp -f target/debug/takd docker/tor-test/takd
strip docker/tor-test/tak docker/tor-test/takd 2>/dev/null
docker build -t tak-tor-test:latest docker/tor-test >/tmp/nodeA_img.txt 2>&1
say "image_exit=$?"

docker rm -f takd-a >/dev/null 2>&1
docker run -d --name takd-a \
  -e RUST_LOG='info,takd=debug,arti_client=info,tor_dirmgr=info,tor_guardmgr=info,tor_hsservice=info,tor_circmgr=info,tor_chanmgr=info,tor_netdir=info' \
  tak-tor-test:latest \
  sh -c 'takd init --node-id node-a --transport tor --pool default && exec takd serve' >/dev/null 2>&1

TOKEN=""
for i in $(seq 1 40); do
  TOKEN=$(docker exec takd-a takd token show 2>/dev/null | head -1)
  case "$TOKEN" in takd:*) break;; *) TOKEN="";; esac
  sleep 5
done
say "onion_token=$([ -n "$TOKEN" ] && echo YES || echo NO)"
say "token_prefix=$(printf '%s' "$TOKEN" | cut -c1-24)"

docker logs takd-a >/tmp/nodeA.log 2>&1
say "log_lines=$(wc -l < /tmp/nodeA.log)"
say "arti_log_lines=$(grep -ciE 'arti|tor_|guard|circuit|chan|consensus|bootstrap|onion|netdir' /tmp/nodeA.log)"
say "err_lines=$(grep -ci 'ERROR' /tmp/nodeA.log)"
say "warn_lines=$(grep -ci 'WARN' /tmp/nodeA.log)"
say "outbound_conns=$(docker exec takd-a sh -c 'cat /proc/net/tcp /proc/net/tcp6 2>/dev/null' | awk 'NR>1 && $4!="0A"' | wc -l)"
say "arti_cache_files=$(docker exec takd-a sh -c 'find /root/.local/state/takd -path "*arti*" -type f 2>/dev/null | wc -l')"
PID=$(docker inspect -f '{{.State.Pid}}' takd-a 2>/dev/null)
fut=0; epo=0; oth=0
for t in /proc/$PID/task/*; do w=$(cat "$t/wchan" 2>/dev/null); case "$w" in *futex*) fut=$((fut+1));; *poll*) epo=$((epo+1));; *) oth=$((oth+1));; esac; done
say "wchan_futex=$fut wchan_poll=$epo wchan_other=$oth"
say "DONE"
