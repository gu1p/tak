#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

is_ignored_by_root_gitignore() {
  local path="$1"
  local path_with_slash="${path%/}/"
  local rooted_path="/${path_with_slash}"

  [[ -f .gitignore ]] || return 1
  grep -Fxq -- "$path_with_slash" .gitignore || grep -Fxq -- "$rooted_path" .gitignore
}

is_ignored() {
  local path="$1"
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git check-ignore -q "$path"
    return
  fi
  is_ignored_by_root_gitignore "$path"
}

require_ignored() {
  local path="$1"
  local description="$2"
  if ! is_ignored "$path"; then
    echo "check_generated_artifacts_ignore: expected ${description} to be ignored (${path})" >&2
    exit 1
  fi
}

require_ignored "dist-manual/" "manual release artifacts"
require_ignored ".tmp/release-target/" "release target cache"
