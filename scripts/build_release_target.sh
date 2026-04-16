#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

target="${1:?target triple is required}"
build_mode="${2:?build mode is required}"
target_dir=".tmp/release-target/${target}"

mkdir -p "$target_dir"

case "$build_mode" in
  build)
    cargo build --release --locked --target "$target" --target-dir "$target_dir" -p tak -p takd
    ;;
  zigbuild)
    cargo zigbuild --release --locked --target "$target" --target-dir "$target_dir" -p tak -p takd
    ;;
  *)
    echo "build_release_target: unsupported build mode ${build_mode}" >&2
    exit 1
    ;;
esac

release_dir="${target_dir}/${target}/release"
test -x "${release_dir}/tak"
test -x "${release_dir}/takd"
