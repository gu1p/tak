#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p .tmp/coverage

rm -rf target
df -h .

bash scripts/run_coverage.sh
