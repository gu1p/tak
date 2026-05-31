#!/usr/bin/env bash
# Two-node real-Tor integration test for tak / takd.
#
#   node A (takd-a)  = takd-only "remote node"      -> publishes an onion service
#   node B (takd-b)  = tak + local takd "bridge"    -> dials A over Tor, runs jobs
#
# MOCK_CONTAINER=true makes the remote worker SIMULATE container execution, so a
# takd node can accept/"run" jobs while living inside a container that has no
# nested Docker/Podman runtime. The point of the test is to exercise the REAL Tor
# transport (onion publish + warm peer session) between the two containers.
#
# Usage:  bash docker/tor-test/run.sh            # full build + launch + probe
#         KEEP=1 bash docker/tor-test/run.sh     # leave containers running
set -uo pipefail
cd "$(dirname "$0")/../.."   # repo root

IMAGE=tak-tor-test:latest
# Verbose enough to SEE Tor bootstrap + connection lifecycle; tune as needed.
RUST_LOG_VAL='info,takd=debug,tak_exec=info,arti_client=info,tor_dirmgr=info,tor_guardmgr=info,tor_hsservice=info,tor_circmgr=info'
WAIT_SECS=300

step(){ printf '\n\033[1;36m== %s ==\033[0m\n' "$*"; }

step "build binaries (host, debug)"
cargo build -p tak -p takd || exit 1

step "stage + strip into image context"
cp -f target/debug/tak  docker/tor-test/tak
cp -f target/debug/takd docker/tor-test/takd
strip docker/tor-test/tak docker/tor-test/takd 2>/dev/null || true

step "build image"
docker build -t "$IMAGE" docker/tor-test || exit 1

step "clean old containers"
docker rm -f takd-a takd-b >/dev/null 2>&1 || true

step "launch node A (takd-only remote node)"
docker run -d --name takd-a -e RUST_LOG="$RUST_LOG_VAL" "$IMAGE" \
  sh -c 'takd init --node-id node-a --transport tor --pool default --tag smoke --capability mock && exec takd serve'

step "launch node B (tak + local takd bridge)"
# B also runs transport=tor: its broker reuses B's hidden-service Tor client to
# dial OUT to A, so B must finish its own onion bootstrap before it can connect.
docker run -d --name takd-b -e RUST_LOG="$RUST_LOG_VAL" "$IMAGE" \
  sh -c 'takd init --node-id node-b --transport tor && exec takd serve'

step "wait for node A onion to publish (capture invite token)"
A_TOKEN=$(docker exec takd-a takd token show --wait --timeout-secs "$WAIT_SECS")
echo "A invite: ${A_TOKEN:-<none/timeout>}"

step "wait for node B onion (broker needs its own client to dial out)"
docker exec takd-b takd token show --wait --timeout-secs "$WAIT_SECS" >/dev/null \
  && echo "B onion ready" || echo "B onion: timeout"

step "register A on B (this PROBES A over Tor)"
docker exec takd-b tak remote add "$A_TOKEN"
docker exec takd-b tak remote list || true

step "peer status — the Tor connectivity proof (expect node-a Connected)"
docker exec takd-b takd peers || true

step "ship THIS repo into B (tracked files only) and run it against A over Tor"
docker exec takd-b sh -c 'rm -rf /work/repo && mkdir -p /work/repo'
git archive HEAD | docker exec -i takd-b tar -x -C /work/repo
# A real repo task; with MOCK_CONTAINER the remote node simulates the container.
docker exec -w /work/repo takd-b tak run fmt-check || true

step "logs — node A"
docker logs takd-a 2>&1 | tail -50
step "logs — node B"
docker logs takd-b 2>&1 | tail -50

if [ "${KEEP:-0}" != "1" ]; then
  step "cleanup (set KEEP=1 to keep containers)"
  docker rm -f takd-a takd-b >/dev/null 2>&1 || true
fi
