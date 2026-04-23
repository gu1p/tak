#!/usr/bin/env bash
set -euo pipefail

target_root="${PWD}/target"

fail() {
  printf '%s\n' "$1" >&2
  exit 1
}

ensure_contains() {
  local needle="$1"
  case " $* " in
    *" ${needle} "*) ;;
    *) fail "missing ${needle} in cargo invocation: $*" ;;
  esac
}

if [ "$#" -lt 1 ]; then
  fail "missing cargo subcommand"
fi

case "$1" in
  llvm-cov)
    shift
    case "${1:-}" in
      clean)
        ensure_contains "--workspace" "$@"
        exit 0
        ;;
      show-env)
        shift
        if [ "$#" -ne 1 ] || [ "$1" != "--sh" ]; then
          fail "unexpected cargo llvm-cov show-env invocation: $*"
        fi
        printf "export LLVM_PROFILE_FILE='%s'\n" "${target_root}/tak-%p-%12m.profraw"
        printf "export CARGO_LLVM_COV_TARGET_DIR='%s'\n" "${target_root}"
        exit 0
        ;;
      report)
        shift
        ensure_contains "--workspace" "$@"
        ensure_contains "--all-features" "$@"
        ensure_contains "--lcov" "$@"
        output_path=""
        prev=""
        for arg in "$@"; do
          if [ "$prev" = "--output-path" ]; then
            output_path="$arg"
          fi
          prev="$arg"
        done
        [ -n "$output_path" ] || fail "missing --output-path in cargo llvm-cov report"
        mkdir -p "$(dirname "${output_path}")"
        printf 'TN:\n' > "${output_path}"
        exit 0
        ;;
      *)
        fail "unexpected cargo llvm-cov subcommand: $1"
        ;;
    esac
    ;;
  build)
    ensure_contains "--all-features" "$@"
    bin=""
    case " $* " in
      *" -p tak --bin tak "*) bin="tak" ;;
      *" -p takd --bin takd "*) bin="takd" ;;
      *) fail "unexpected cargo build invocation: $*" ;;
    esac
    mkdir -p "${target_root}/debug"
    cat > "${target_root}/debug/${bin}" <<EOF
#!/usr/bin/env bash
exit 0
EOF
    chmod +x "${target_root}/debug/${bin}"
    exit 0
    ;;
  test)
    ensure_contains "--workspace" "$@"
    ensure_contains "--all-features" "$@"
    [ "${TAK_TEST_TAK_BIN:-}" = "${target_root}/debug/tak" ] \
      || fail "unexpected TAK_TEST_TAK_BIN: ${TAK_TEST_TAK_BIN:-unset}"
    [ "${TAK_TEST_TAKD_BIN:-}" = "${target_root}/debug/takd" ] \
      || fail "unexpected TAK_TEST_TAKD_BIN: ${TAK_TEST_TAKD_BIN:-unset}"
    [ -x "${TAK_TEST_TAK_BIN}" ] || fail "missing tak test binary"
    [ -x "${TAK_TEST_TAKD_BIN}" ] || fail "missing takd test binary"
    exit 0
    ;;
  *)
    fail "unexpected cargo invocation: $*"
    ;;
esac
