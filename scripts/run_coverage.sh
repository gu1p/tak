#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p .tmp/coverage

cargo llvm-cov clean --workspace
cargo llvm-cov \
  --workspace \
  --all-features \
  --fail-under-lines 75 \
  --lcov \
  --output-path .tmp/coverage/lcov.info
