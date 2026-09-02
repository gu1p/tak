#!/usr/bin/env bash
set -euo pipefail

if tak runs list >/dev/null 2>&1; then
  echo "local takd is already ready"
  exit 0
fi

log_dir="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
log_path="${log_dir%/}/takd-local-${GITHUB_JOB:-workflow}.log"
mkdir -p "$log_dir"

nohup takd serve >"$log_path" 2>&1 &
daemon_pid=$!

attempt=0
while [ "$attempt" -lt 300 ]; do
  if tak runs list >/dev/null 2>&1; then
    echo "local takd is ready (pid=${daemon_pid}, log=${log_path})"
    exit 0
  fi
  if ! kill -0 "$daemon_pid" 2>/dev/null; then
    echo "local takd exited before becoming ready; log follows:" >&2
    tail -n 200 "$log_path" >&2 || true
    exit 1
  fi
  attempt=$((attempt + 1))
  sleep 0.1
done

echo "local takd did not become ready; log follows:" >&2
tail -n 200 "$log_path" >&2 || true
exit 1
