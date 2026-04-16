#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

target="${1:?target triple is required}"
tag="${TAK_RELEASE_TAG:-manual}"
target_dir=".tmp/release-target/${target}"
release_dir="${target_dir}/${target}/release"
tak_bin="${release_dir}/tak"
takd_bin="${release_dir}/takd"
dist_root="dist-manual"
pkg_dir="${dist_root}/pkg/${target}"

test -x "$tak_bin"
test -x "$takd_bin"

if [[ -n "${TAK_BUILD_VERSION:-}" ]]; then
  expected_version="tak ${TAK_BUILD_VERSION}"
  actual_version="$("$tak_bin" --version | tr -d '\r')"
  if [[ "$actual_version" != "$expected_version" ]]; then
    echo "package_release_target: version mismatch for ${target}" >&2
    echo "expected: ${expected_version}" >&2
    echo "actual:   ${actual_version}" >&2
    exit 1
  fi
fi

rm -rf "$pkg_dir"
mkdir -p "$pkg_dir"
cp "$tak_bin" "${pkg_dir}/tak"
cp "$takd_bin" "${pkg_dir}/takd"
chmod +x "${pkg_dir}/tak" "${pkg_dir}/takd"

archive="tak-${tag}-${target}.tar.gz"
mkdir -p "$dist_root"
tar -C "$pkg_dir" -czf "${dist_root}/${archive}" tak takd
(
  cd "$dist_root"
  shasum -a 256 "${archive}" > "${archive}.sha256"
)
