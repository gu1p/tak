#!/usr/bin/env bash
set -euo pipefail

readonly RUST_TOOLCHAIN="1.97.1"
readonly HAWK_REPOSITORY="https://github.com/gu1p/hawk"
readonly HAWK_REVISION="98efa9f7590d12672ece0527e4a908788792a997"
readonly HAWK_REVISION_SHORT="${HAWK_REVISION:0:7}"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
WORKSPACE_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
readonly WORKSPACE_ROOT

readonly TEST_TMPDIR="${TAK_TEST_TMPDIR:-/var/tmp/tak-tests}"
readonly CARGO_TOOLS_DIR="${WORKSPACE_ROOT}/.tmp/cargo-tools"
readonly CARGO_TARGET_DIR="${WORKSPACE_ROOT}/.tmp/cargo-target-local"
readonly RUSTUP_HOME="${WORKSPACE_ROOT}/.tmp/rustup-home"

prepare_environment() {
    mkdir -p \
        "${TEST_TMPDIR}" \
        "${CARGO_TOOLS_DIR}" \
        "${CARGO_TARGET_DIR}" \
        "${RUSTUP_HOME}"

    export TMPDIR="${TEST_TMPDIR}"
    export CARGO_INSTALL_ROOT="${CARGO_TOOLS_DIR}"
    export CARGO_TARGET_DIR
    export RUSTUP_HOME
    export PATH="${CARGO_TOOLS_DIR}/bin:${PATH}"
}

install_rustc_dev() {
    if rustup component list --toolchain "${RUST_TOOLCHAIN}" --installed 2>/dev/null \
        | grep -q '^rustc-dev-'; then
        return
    fi

    rustup toolchain install "${RUST_TOOLCHAIN}" --component rustc-dev
}

install_hawk() {
    if cargo +"${RUST_TOOLCHAIN}" install --list 2>/dev/null \
        | grep -q "${HAWK_REVISION_SHORT}"; then
        return
    fi

    RUSTC_BOOTSTRAP=1 cargo +"${RUST_TOOLCHAIN}" install \
        --locked \
        --force \
        --git "${HAWK_REPOSITORY}" \
        --rev "${HAWK_REVISION}" \
        cargo-hawk
}

install_tools() {
    install_rustc_dev
    install_hawk
}

configure_rustc_driver_library_path() {
    local rustc_sysroot
    rustc_sysroot="$(rustc +"${RUST_TOOLCHAIN}" --print sysroot)"

    case "$(uname -s)" in
        Darwin)
            export DYLD_LIBRARY_PATH="${rustc_sysroot}/lib${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}"
            ;;
        *)
            export LD_LIBRARY_PATH="${rustc_sysroot}/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
            ;;
    esac
}

check_dead_code() {
    configure_rustc_driver_library_path
    CARGO_BUILD_RUSTC_WRAPPER="" cargo +"${RUST_TOOLCHAIN}" hawk check \
        --only test-only \
        -D hawk::test_only
}

usage() {
    echo "Usage: $0 <install|check>" >&2
}

main() {
    prepare_environment

    case "${1:-}" in
        install)
            install_tools
            ;;
        check)
            check_dead_code
            ;;
        *)
            usage
            return 2
            ;;
    esac
}

main "$@"
