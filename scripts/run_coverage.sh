#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p .tmp/coverage

cargo llvm-cov clean --workspace

source <(cargo llvm-cov show-env --export-prefix --sh)

cargo build --all-features -p tak --bin tak
cargo build --all-features -p takd --bin takd

export TAK_TEST_TAK_BIN="${CARGO_TARGET_DIR}/debug/tak"
export TAK_TEST_TAKD_BIN="${CARGO_TARGET_DIR}/debug/takd"

cargo test \
  --workspace \
  --all-features

cargo llvm-cov report \
  --workspace \
  --all-features \
  --fail-under-lines 75 \
  --lcov \
  --output-path .tmp/coverage/lcov.info
