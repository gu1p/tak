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
require_match "$CI_WORKFLOW" 'name:\s*Install Tak from checkout' "CI source Tak bootstrap step"
require_match "$CI_WORKFLOW" 'cargo build --locked --bins -p tak -p takd' "CI source Tak build command"
require_match "$CI_WORKFLOW" 'echo "\$PWD/target/debug" >> "\$GITHUB_PATH"' "CI source Tak path export"
require_match "$CI_WORKFLOW" 'GITHUB_PATH' "CI Tak path export"
require_match "$CI_WORKFLOW" 'tak --version' "CI Tak version smoke check"
require_match "$CI_WORKFLOW" 'name:\s*Install ripgrep' "CI ripgrep install step"
require_match "$CI_WORKFLOW" 'sudo apt-get update' "CI apt metadata refresh for ripgrep"
require_match "$CI_WORKFLOW" 'sudo apt-get install -y ripgrep' "CI ripgrep package install"
require_match "$CI_WORKFLOW" 'tak run //:ci' "CI Tak ci command"
require_match "$CI_WORKFLOW" 'tak run "\$\{\{ matrix\.tak_task \}\}"' "CI Tak matrix build command"
require_no_match "$CI_WORKFLOW" 'os:\s*macos-13' "CI deprecated macOS x64 runner"
require_no_match "$CI_WORKFLOW" 'os:\s*macos-14' "CI deprecated macOS arm runner"
require_match "$CI_WORKFLOW" 'os:\s*macos-15-intel' "CI latest macOS x64 runner"
require_match "$CI_WORKFLOW" 'os:\s*macos-latest' "CI latest macOS arm runner"
require_match "$CI_WORKFLOW" 'os:\s*macos-15-intel\s*\n\s*target:\s*x86_64-apple-darwin' "CI x86_64 macOS runner mapping"
require_match "$CI_WORKFLOW" 'os:\s*macos-latest\s*\n\s*target:\s*aarch64-apple-darwin' "CI aarch64 macOS runner mapping"
require_match "$CI_WORKFLOW" 'tak_task:\s*//:build-release-x86_64-unknown-linux-musl' "CI Tak task for linux x86_64 build"
require_match "$CI_WORKFLOW" 'tak_task:\s*//:build-release-aarch64-unknown-linux-musl' "CI Tak task for linux aarch64 build"
require_match "$CI_WORKFLOW" 'tak_task:\s*//:build-release-x86_64-apple-darwin' "CI Tak task for macOS x86_64 build"
require_match "$CI_WORKFLOW" 'tak_task:\s*//:build-release-aarch64-apple-darwin' "CI Tak task for macOS arm64 build"
require_no_match "$CI_WORKFLOW" '\bcargo run --locked -p tak -- run\b' "CI cargo-run Tak bootstrap"
require_no_match "$CI_WORKFLOW" 'cargo test --workspace' "CI direct workspace tests"
require_no_match "$CI_WORKFLOW" 'cargo clippy --workspace --all-targets -- -D warnings' "CI direct clippy invocation"
require_no_match "$CI_WORKFLOW" 'cargo fmt --all -- --check' "CI direct rustfmt invocation"
require_no_match "$CI_WORKFLOW" '\./get-tak\.sh' "CI release-download Tak bootstrap"
require_no_match "$CI_WORKFLOW" 'TAK_INSTALL_DIR:' "CI release-download Tak install dir"
require_no_match "$CI_WORKFLOW" 'tak run //:check' "CI separate Tak check command"
require_no_match "$CI_WORKFLOW" 'tak run //:coverage' "CI separate Tak coverage command"
require_no_match "$CI_WORKFLOW" 'name:\s*Reclaim disk before coverage' "CI standalone disk-reclaim step before coverage"
require_no_match "$CI_WORKFLOW" '\bmake\b' "CI make usage"

require_match "$RELEASE_WORKFLOW" "push:" "Release push trigger"
require_match "$RELEASE_WORKFLOW" "branches:" "Release push branch filter"
require_match "$RELEASE_WORKFLOW" "- '\\*\\*'" "Release all-branches wildcard push filter"
require_no_match "$RELEASE_WORKFLOW" "workflow_run:" "Release workflow_run trigger"
require_no_match "$RELEASE_WORKFLOW" 'os:\s*macos-13' "Release deprecated macOS x64 runner"
require_no_match "$RELEASE_WORKFLOW" 'os:\s*macos-14' "Release deprecated macOS arm runner"
require_match "$RELEASE_WORKFLOW" 'os:\s*macos-15-intel' "Release latest macOS x64 runner"
require_match "$RELEASE_WORKFLOW" 'os:\s*macos-latest' "Release latest macOS arm runner"
require_match "$RELEASE_WORKFLOW" 'os:\s*macos-15-intel\s*\n\s*target:\s*x86_64-apple-darwin' "Release x86_64 macOS runner mapping"
require_match "$RELEASE_WORKFLOW" 'os:\s*macos-latest\s*\n\s*target:\s*aarch64-apple-darwin' "Release aarch64 macOS runner mapping"
require_no_match "$RELEASE_WORKFLOW" "head_branch == '" "Release branch allowlist condition"
require_match "$RELEASE_WORKFLOW" 'tag="v\$\{workspace_version\}-\$\{head_sha:0:8\}"' "Release per-commit tag with short SHA"
require_match "$RELEASE_WORKFLOW" 'tag="v\$\{workspace_version\}-\$\{head_sha:0:12\}"' "Release collision fallback tag with 12-char SHA"
require_match "$RELEASE_WORKFLOW" 'tag="v\$\{workspace_version\}-\$\{head_sha\}"' "Release collision fallback tag with full SHA"
require_match "$RELEASE_WORKFLOW" 'TAK_RELEASE_TAG: \$\{\{ needs\.prepare_tag\.outputs\.tag \}\}' "Release Tak archive tag env"
require_match "$RELEASE_WORKFLOW" 'TAK_BUILD_VERSION: \$\{\{ needs\.prepare_tag\.outputs\.version \}\}' "Release build-time TAK version injection"
require_match "$RELEASE_WORKFLOW" 'timeout-minutes:\s*240' "Release extended binary build timeout"
require_match "$RELEASE_WORKFLOW" 'GH_REPO: \$\{\{ github\.repository \}\}' "Release gh CLI repository context"
require_match "$RELEASE_WORKFLOW" 'name:\s*Install Tak from checkout' "Release source Tak bootstrap step"
require_match "$RELEASE_WORKFLOW" 'cargo build --locked --bins -p tak -p takd' "Release source Tak build command"
require_match "$RELEASE_WORKFLOW" 'echo "\$PWD/target/debug" >> "\$GITHUB_PATH"' "Release source Tak path export"
require_match "$RELEASE_WORKFLOW" 'GITHUB_PATH' "Release Tak path export"
require_match "$RELEASE_WORKFLOW" 'tak --version' "Release Tak version smoke check"
require_match "$RELEASE_WORKFLOW" 'tak run "\$\{\{ matrix\.tak_task \}\}"' "Release Tak package command"
require_match "$RELEASE_WORKFLOW" 'tak_task:\s*//:package-release-x86_64-unknown-linux-musl' "Release Tak task for linux x86_64 package"
require_match "$RELEASE_WORKFLOW" 'tak_task:\s*//:package-release-aarch64-unknown-linux-musl' "Release Tak task for linux aarch64 package"
require_match "$RELEASE_WORKFLOW" 'tak_task:\s*//:package-release-x86_64-apple-darwin' "Release Tak task for macOS x86_64 package"
require_match "$RELEASE_WORKFLOW" 'tak_task:\s*//:package-release-aarch64-apple-darwin' "Release Tak task for macOS arm64 package"
require_match "$RELEASE_WORKFLOW" 'dist-manual/\*\.tar\.gz' "Release upload includes Tak-built binary archives only"
require_match "$RELEASE_WORKFLOW" 'dist-manual/\*\.tar\.gz\.sha256' "Release upload includes Tak-built archive checksums only"
require_no_match "$RELEASE_WORKFLOW" '\bcargo run --locked -p tak -- run\b' "Release cargo-run Tak bootstrap"
require_no_match "$RELEASE_WORKFLOW" '\./get-tak\.sh' "Release release-download Tak bootstrap"
require_no_match "$RELEASE_WORKFLOW" 'TAK_INSTALL_DIR:' "Release release-download Tak install dir"
require_no_match "$RELEASE_WORKFLOW" 'cargo build --release --locked --target "\$\{\{ matrix\.target \}\}" -p tak -p takd' "Release direct native build command"
require_no_match "$RELEASE_WORKFLOW" 'cargo zigbuild --release --locked --target "\$\{\{ matrix\.target \}\}" -p tak -p takd' "Release direct zigbuild command"
require_match "$RELEASE_WORKFLOW" '--prerelease' "Release prerelease flag for non-main branches"
require_no_match "$RELEASE_WORKFLOW" '\bmake\b' "Release make usage"

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
