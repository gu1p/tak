#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

require_ignored() {
  local path="$1"
  local description="$2"
  if ! git check-ignore -q "$path"; then
    echo "check_generated_artifacts_ignore: expected ${description} to be ignored (${path})" >&2
    exit 1
  fi
}

require_ignored "dist-manual/" "manual release artifacts"
require_ignored ".tmp/release-target/" "release target cache"
