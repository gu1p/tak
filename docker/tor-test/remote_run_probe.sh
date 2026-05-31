#!/usr/bin/env bash
# Disciplined single-file probe: copy the minimal remote-eligible project into B,
# run it remotely against A over Tor, and capture EVERYTHING to one report file.
set -uo pipefail
cd /media/gp/Projects1/gu1p/tak
REP=/tmp/remote_run_report.txt; : > "$REP"
say(){ printf '%s\n' "$*" >> "$REP"; }
dump(){ say "----- $1 -----"; shift; "$@" >>"$REP" 2>&1; say "[rc=$?]"; }

# Ship the minimal project into B
docker exec takd-b sh -c 'rm -rf /work/probe && mkdir -p /work/probe'
docker cp docker/tor-test/remote_probe_project/TASKS.py takd-b:/work/probe/TASKS.py

say "=== peers BEFORE run ==="
docker exec takd-b takd peers >>"$REP" 2>&1

dump "tak list (probe project)" docker exec -w /work/probe takd-b tak list
dump "tak run hello-remote --remote" docker exec -w /work/probe takd-b tak run hello-remote --remote

say "=== peers AFTER run ==="
docker exec takd-b takd peers >>"$REP" 2>&1

say "=== A service.log: submit/worker/exec lines ==="
docker exec takd-a sh -c 'grep -iE "submit|worker|exec|TAK_REMOTE_OK_MARKER|mock|simulat|/v1/" /root/.local/state/takd/service.log 2>/dev/null | tail -30' >>"$REP" 2>&1

say "=== B service.log: place/broker/peer/submit lines ==="
docker exec takd-b sh -c 'grep -iE "place|broker|peer|submit|remote|dial|http2|select|node-a" /root/.local/state/takd/service.log 2>/dev/null | tail -40' >>"$REP" 2>&1

say "DONE"
