#!/usr/bin/env bash
# Real-Tor two-role relay test:
#   tak (local container) -> local takd -> Tor/HTTP2 -> remote takd
set -euo pipefail

cd "$(dirname "$0")/../.."

REMOTE_IMAGE="${TAK_TOR_E2E_REMOTE_IMAGE:-tak-tor-remote:latest}"
LOCAL_IMAGE="${TAK_TOR_E2E_LOCAL_IMAGE:-tak-tor-local:latest}"
REMOTE_CONTAINER="${TAK_TOR_E2E_REMOTE_CONTAINER:-tak-tor-remote}"
LOCAL_CONTAINER="${TAK_TOR_E2E_LOCAL_CONTAINER:-tak-tor-local}"
TASK="${TAK_TOR_E2E_TASK:-generated-artifact-ignore-check}"
WAIT_SECS="${TAK_TOR_E2E_WAIT_SECS:-300}"
LOG_DIR="${TAK_TOR_E2E_LOG_DIR:-.tmp/tor-e2e/$(date +%Y%m%d-%H%M%S)}"
RUST_LOG_VAL="${TAK_TOR_E2E_RUST_LOG:-info,takd=debug,tak_exec=info,tor_hsservice=warn,tor_dirmgr=warn,tor_guardmgr=warn,tor_circmgr=warn}"

step() { printf '\n\033[1;36m== %s ==\033[0m\n' "$*"; }

mkdir -p "$LOG_DIR"

capture_logs() {
  mkdir -p "$LOG_DIR"
  docker logs "$REMOTE_CONTAINER" >"$LOG_DIR/remote-docker.log" 2>&1 || true
  docker logs "$LOCAL_CONTAINER" >"$LOG_DIR/local-docker.log" 2>&1 || true
  docker cp "$REMOTE_CONTAINER:/root/.local/state/takd/service.log" "$LOG_DIR/remote-service.log" >/dev/null 2>&1 || true
  docker cp "$LOCAL_CONTAINER:/root/.local/state/takd/service.log" "$LOG_DIR/local-service.log" >/dev/null 2>&1 || true
}

cleanup() {
  capture_logs
  if [[ "${KEEP:-0}" != "1" ]]; then
    docker rm -f "$REMOTE_CONTAINER" "$LOCAL_CONTAINER" >/dev/null 2>&1 || true
  fi
  printf '\nlogs: %s\n' "$LOG_DIR"
}
trap cleanup EXIT

step "build host binaries"
cargo build -p tak -p takd

step "stage binaries into docker context"
cp -f target/debug/takd docker/tor-test/takd
cp -f target/debug/tak docker/tor-test/tak
strip docker/tor-test/takd docker/tor-test/tak 2>/dev/null || true

step "build role-specific images"
docker build -f docker/tor-test/Dockerfile.remote -t "$REMOTE_IMAGE" docker/tor-test
docker build -f docker/tor-test/Dockerfile.local -t "$LOCAL_IMAGE" docker/tor-test

step "start remote takd-only node"
docker rm -f "$REMOTE_CONTAINER" "$LOCAL_CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$REMOTE_CONTAINER" -e RUST_LOG="$RUST_LOG_VAL" "$REMOTE_IMAGE" \
  sh -c 'takd init --node-id node-a --transport tor --pool default --tag smoke --capability mock && exec takd serve' >/dev/null

step "wait for remote onion token"
REMOTE_TOKEN="$(docker exec "$REMOTE_CONTAINER" takd token show --wait --timeout-secs "$WAIT_SECS" 2>/dev/null | grep -m1 '^takd:')"
if [[ -z "$REMOTE_TOKEN" ]]; then
  echo "remote node did not produce a takd token" >&2
  exit 1
fi
printf '%s\n' "$REMOTE_TOKEN" >"$LOG_DIR/remote-token.txt"
docker exec "$REMOTE_CONTAINER" sh -c "grep -ao 'http://[^ ]*\\.onion' /root/.local/state/takd/service.log 2>/dev/null | tail -1" >"$LOG_DIR/remote-onion.txt" || true
printf 'remote onion: %s\n' "$(cat "$LOG_DIR/remote-onion.txt" 2>/dev/null || true)"

step "start local tak + takd bridge"
docker run -d --name "$LOCAL_CONTAINER" -e RUST_LOG="$RUST_LOG_VAL" "$LOCAL_IMAGE" \
  sh -c 'takd init --node-id node-b --transport tor && exec takd serve' >/dev/null
docker exec "$LOCAL_CONTAINER" takd token show --wait --timeout-secs "$WAIT_SECS" >/dev/null 2>&1

step "register remote node in local container"
docker exec "$LOCAL_CONTAINER" tak remote add "$REMOTE_TOKEN"
docker exec "$LOCAL_CONTAINER" tak remote list | tee "$LOG_DIR/remote-list.txt"

step "wait for local takd to warm a Tor peer session"
connected=no
for i in $(seq 1 40); do
  docker exec "$LOCAL_CONTAINER" takd peers >"$LOG_DIR/peers-latest.txt" 2>&1 || true
  if grep -qiw connected "$LOG_DIR/peers-latest.txt"; then
    connected=yes
    break
  fi
  sleep 5
done
cat "$LOG_DIR/peers-latest.txt"
if [[ "$connected" != "yes" ]]; then
  echo "node-a did not become connected through local takd" >&2
  exit 1
fi

step "copy current repository into local-client container"
docker exec "$LOCAL_CONTAINER" sh -c 'rm -rf /work/repo && mkdir -p /work/repo'
git ls-files -z --cached --others --exclude-standard \
  | tar --null -T - -cf - \
  | docker exec -i "$LOCAL_CONTAINER" tar -x -C /work/repo

step "run realistic repo task through local takd relay"
set +e
docker exec -w /work/repo "$LOCAL_CONTAINER" tak run "$TASK" --remote 2>&1 | tee "$LOG_DIR/tak-run.log"
run_rc=${PIPESTATUS[0]}
if [[ "$run_rc" -eq 0 ]]; then
  docker exec -w /work/repo "$LOCAL_CONTAINER" \
    tak exec --remote --container-image alpine:3.20 -- \
    bash -lc 'test -f Cargo.toml && test -f crates/takd/src/service.rs && echo TAK_TOR_E2E_REPO_PROBE_OK' \
    2>&1 | tee -a "$LOG_DIR/tak-run.log"
  run_rc=${PIPESTATUS[0]}
fi
set -e
capture_logs
if [[ "$run_rc" -ne 0 ]]; then
  echo "tak run failed with exit code $run_rc" >&2
  exit "$run_rc"
fi

step "verify relay path appeared in logs"
grep -q "placing remote task through Tor peer" "$LOG_DIR/local-service.log"
grep -q "forwarding workspace upload stream over Tor" "$LOG_DIR/local-service.log"
grep -q "forwarding remote HTTP request over Tor" "$LOG_DIR/local-service.log"
grep -q "workspace upload stream committed" "$LOG_DIR/remote-service.log"
grep -q "remote submit received" "$LOG_DIR/remote-service.log"
grep -q "remote worker task finished" "$LOG_DIR/remote-service.log"
echo "verified path: tak -> local takd -> Tor/HTTP2 -> remote takd"
