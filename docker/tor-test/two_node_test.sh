#!/usr/bin/env bash
# Full two-node real-Tor integration test (build assumed already done & staged).
# Writes a compact, greppable progress file to /tmp/twonode_sum.txt and keeps the
# full per-node logs in /tmp/twonode_{a,b}.log.
#
# Flow: launch A (remote node) + B (client+bridge) -> wait A onion -> register A
# on B (probes A over Tor) -> takd peers (connectivity proof) -> ship this repo
# into B and run a task remotely (MOCK_CONTAINER simulates the container).
set -uo pipefail
cd /media/gp/Projects1/gu1p/tak
SUM=/tmp/twonode_sum.txt; : > "$SUM"
say(){ echo "$*" >> "$SUM"; }
RL='info,takd=debug,tak_exec=info,arti_client=info,tor_dirmgr=warn,tor_hsservice=info'
WAIT=300

docker rm -f takd-a takd-b >/dev/null 2>&1

docker run -d --name takd-a -e RUST_LOG="$RL" tak-tor-test:latest \
  sh -c 'takd init --node-id node-a --transport tor --pool default --tag smoke --capability mock && exec takd serve' >/dev/null 2>&1
say "A_launched=$?"
docker run -d --name takd-b -e RUST_LOG="$RL" tak-tor-test:latest \
  sh -c 'takd init --node-id node-b --transport tor && exec takd serve' >/dev/null 2>&1
say "B_launched=$?"

# A's invite (blocks until A's onion is published and self-probe passes)
A_TOKEN=$(docker exec takd-a takd token show --wait --timeout-secs "$WAIT" 2>/dev/null | head -1)
say "A_onion_ready=$([ "${A_TOKEN#takd:}" != "$A_TOKEN" ] && echo YES || echo NO)"
say "A_token_prefix=$(printf '%s' "$A_TOKEN" | cut -c1-26)"

# B must finish its own onion bootstrap before its broker can dial OUT.
docker exec takd-b takd token show --wait --timeout-secs "$WAIT" >/dev/null 2>&1
say "B_onion_ready=$?"

# Register A on B — this performs a real Tor probe of A from B.
docker exec takd-b tak remote add "$A_TOKEN" >/tmp/twonode_add.txt 2>&1
say "remote_add_rc=$?"
docker exec takd-b tak remote list >/tmp/twonode_list.txt 2>&1
say "remote_list_rc=$?"

# Give the daemon a few heartbeat cycles to reach Connected, then snapshot peers.
sleep 20
docker exec takd-b takd peers >/tmp/twonode_peers.txt 2>&1
say "peers_rc=$?"
say "peers_has_node_a=$(grep -c node-a /tmp/twonode_peers.txt)"
say "peers_connected=$(grep -ci 'connected' /tmp/twonode_peers.txt)"

# Ship THIS repo (tracked files) into B and run a task remotely against A.
docker exec takd-b sh -c 'rm -rf /work/repo && mkdir -p /work/repo'
git archive HEAD | docker exec -i takd-b tar -x -C /work/repo
docker exec -w /work/repo takd-b tak run line-limits-check --remote >/tmp/twonode_run.txt 2>&1
say "remote_run_rc=$?"

docker logs takd-a >/tmp/twonode_a.log 2>&1
docker logs takd-b >/tmp/twonode_b.log 2>&1
say "A_log_lines=$(wc -l < /tmp/twonode_a.log)"
say "B_log_lines=$(wc -l < /tmp/twonode_b.log)"
say "DONE"
