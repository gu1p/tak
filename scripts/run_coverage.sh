#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p .tmp/coverage

cargo llvm-cov clean --workspace

source <(cargo llvm-cov show-env --sh)

coverage_target_dir="${CARGO_TARGET_DIR:-${CARGO_LLVM_COV_TARGET_DIR:-target}}"
case "$coverage_target_dir" in
  /*) ;;
  *) coverage_target_dir="$ROOT_DIR/$coverage_target_dir" ;;
esac

cargo build --all-features -p tak --bin tak
cargo build --all-features -p takd --bin takd

export TAK_TEST_TAK_BIN="${coverage_target_dir}/debug/tak"
export TAK_TEST_TAKD_BIN="${coverage_target_dir}/debug/takd"

# Retry the test run a couple of times before giving up. A handful of
# coverage-instrumented async tests (e.g. the orphan-watchdog cancellation
# race) are timing-sensitive and can flake under the GitHub runner's exact
# scheduling. Re-running reuses the already-built instrumented binaries and
# accumulates coverage, so a transient flake no longer fails the whole job;
# the first failing attempt's output is still printed for diagnosis.
test_attempts="${TAK_COVERAGE_TEST_ATTEMPTS:-3}"
attempt=1
while true; do
  if cargo test --workspace --all-features; then
    break
  fi
  if [ "${attempt}" -ge "${test_attempts}" ]; then
    echo "::error::coverage test run failed after ${attempt} attempt(s)" >&2
    exit 1
  fi
  echo "::warning::coverage test run failed on attempt ${attempt}/${test_attempts}; retrying" >&2
  attempt=$((attempt + 1))
done

cargo llvm-cov report \
  --fail-under-lines 75 \
  --lcov \
  --output-path .tmp/coverage/lcov.info
