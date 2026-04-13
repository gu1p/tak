#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CI_WORKFLOW="${ROOT_DIR}/.github/workflows/ci.yml"
RELEASE_WORKFLOW="${ROOT_DIR}/.github/workflows/release.yml"

if ! command -v rg >/dev/null 2>&1; then
  echo "check_workflow_binary_matrix: rg is required" >&2
  exit 1
fi

require_match() {
  local file="$1"
  local pattern="$2"
  local description="$3"
  if ! rg -q --multiline -- "$pattern" "$file"; then
    echo "check_workflow_binary_matrix: missing ${description} in ${file}" >&2
    exit 1
  fi
}

require_no_match() {
  local file="$1"
  local pattern="$2"
  local description="$3"
  if rg -q --multiline -- "$pattern" "$file"; then
    echo "check_workflow_binary_matrix: unexpected ${description} in ${file}" >&2
    exit 1
  fi
}

extract_targets() {
  local file="$1"
  rg -o 'target:\s*[A-Za-z0-9_.-]+' "$file" | sed -E 's/.*target:\s*//' | sed -E 's/^[[:space:]]+|[[:space:]]+$//g' | sort -u
}

expected_targets=$'aarch64-apple-darwin\naarch64-unknown-linux-musl\nx86_64-apple-darwin\nx86_64-unknown-linux-musl'

require_match "$CI_WORKFLOW" "push:" "CI push trigger"
require_match "$CI_WORKFLOW" "branches:" "CI push branch filter"
require_match "$CI_WORKFLOW" "- '\\*\\*'" "CI all-branches wildcard push filter"
require_match "$CI_WORKFLOW" '^  build_binaries:' "CI build_binaries job"
require_match "$CI_WORKFLOW" 'timeout-minutes:\s*240' "CI extended binary build timeout"
require_match "$CI_WORKFLOW" 'cargo build --release --locked --target "\$\{\{ matrix\.target \}\}" -p tak -p takd' "CI native build command for tak and takd"
require_match "$CI_WORKFLOW" 'cargo zigbuild --release --locked --target "\$\{\{ matrix\.target \}\}" -p tak -p takd' "CI zigbuild command for tak and takd"
require_no_match "$CI_WORKFLOW" 'os:\s*macos-13' "CI deprecated macOS x64 runner"
require_match "$CI_WORKFLOW" 'name:\s*Reclaim disk before coverage' "CI disk-reclaim step before coverage"
require_match "$CI_WORKFLOW" 'rm -rf target' "CI target cleanup before coverage"
require_match "$CI_WORKFLOW" 'cargo llvm-cov clean --workspace' "CI llvm-cov cleanup before coverage"

require_match "$RELEASE_WORKFLOW" "push:" "Release push trigger"
require_match "$RELEASE_WORKFLOW" "branches:" "Release push branch filter"
require_match "$RELEASE_WORKFLOW" "- '\\*\\*'" "Release all-branches wildcard push filter"
require_no_match "$RELEASE_WORKFLOW" "workflow_run:" "Release workflow_run trigger"
require_no_match "$RELEASE_WORKFLOW" 'os:\s*macos-13' "Release deprecated macOS x64 runner"
require_no_match "$RELEASE_WORKFLOW" "head_branch == '" "Release branch allowlist condition"
require_match "$RELEASE_WORKFLOW" 'tag="v\$\{workspace_version\}-\$\{head_sha:0:8\}"' "Release per-commit tag with short SHA"
require_match "$RELEASE_WORKFLOW" 'tag="v\$\{workspace_version\}-\$\{head_sha:0:12\}"' "Release collision fallback tag with 12-char SHA"
require_match "$RELEASE_WORKFLOW" 'tag="v\$\{workspace_version\}-\$\{head_sha\}"' "Release collision fallback tag with full SHA"
require_match "$RELEASE_WORKFLOW" 'TAK_BUILD_VERSION: \$\{\{ needs\.prepare_tag\.outputs\.version \}\}' "Release build-time TAK version injection"
require_match "$RELEASE_WORKFLOW" 'timeout-minutes:\s*240' "Release extended binary build timeout"
require_match "$RELEASE_WORKFLOW" 'GH_REPO: \$\{\{ github\.repository \}\}' "Release gh CLI repository context"
require_match "$RELEASE_WORKFLOW" 'cargo build --release --locked --target "\$\{\{ matrix\.target \}\}" -p tak -p takd' "Release native build command for tak and takd"
require_match "$RELEASE_WORKFLOW" 'cargo zigbuild --release --locked --target "\$\{\{ matrix\.target \}\}" -p tak -p takd' "Release zigbuild command for tak and takd"
require_match "$RELEASE_WORKFLOW" 'target/\$\{target\}/release/tak --version' "Release tak --version verification step"
require_match "$RELEASE_WORKFLOW" 'cd dist' "Release checksum generation runs from dist directory"
require_match "$RELEASE_WORKFLOW" 'shasum -a 256 "\$\{archive\}" > "\$\{archive\}\.sha256"' "Release checksum manifest uses bare archive name"
require_no_match "$RELEASE_WORKFLOW" 'shasum -a 256 "dist/\$\{archive\}" > "dist/\$\{archive\}\.sha256"' "Release checksum manifest embeds dist path"
require_match "$RELEASE_WORKFLOW" 'dist/\*\.tar\.gz' "Release upload includes binary archives only"
require_match "$RELEASE_WORKFLOW" 'dist/\*\.tar\.gz\.sha256' "Release upload includes archive checksums only"
require_match "$RELEASE_WORKFLOW" '--prerelease' "Release prerelease flag for non-main branches"

ci_targets="$(extract_targets "$CI_WORKFLOW")"
release_targets="$(extract_targets "$RELEASE_WORKFLOW")"

if [ "$ci_targets" != "$expected_targets" ]; then
  echo "check_workflow_binary_matrix: CI matrix targets do not match expected set" >&2
  echo "expected:" >&2
  printf '%s\n' "$expected_targets" >&2
  echo "actual:" >&2
  printf '%s\n' "$ci_targets" >&2
  exit 1
fi

if [ "$release_targets" != "$expected_targets" ]; then
  echo "check_workflow_binary_matrix: Release matrix targets do not match expected set" >&2
  echo "expected:" >&2
  printf '%s\n' "$expected_targets" >&2
  echo "actual:" >&2
  printf '%s\n' "$release_targets" >&2
  exit 1
fi

if [ "$ci_targets" != "$release_targets" ]; then
  echo "check_workflow_binary_matrix: CI and Release matrix targets differ" >&2
  exit 1
fi
