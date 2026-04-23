#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p .tmp/coverage

cargo llvm-cov clean --workspace

source <(cargo llvm-cov show-env --sh)

coverage_target_dir="${CARGO_TARGET_DIR:-${CARGO_LLVM_COV_TARGET_DIR:-target}}"

cargo build --all-features -p tak --bin tak
cargo build --all-features -p takd --bin takd

export TAK_TEST_TAK_BIN="${coverage_target_dir}/debug/tak"
export TAK_TEST_TAKD_BIN="${coverage_target_dir}/debug/takd"

cargo test \
  --workspace \
  --all-features

cargo llvm-cov report \
  --fail-under-lines 75 \
  --lcov \
  --output-path .tmp/coverage/lcov.info
