#!/usr/bin/env bash
set -euo pipefail

declare -a CARGO_CMD=()

err() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

repo_root() {
  local script_dir
  script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
  printf '%s' "$script_dir"
}

install_dir() {
  local candidate

  if [[ -n "${TAK_INSTALL_DIR:-}" ]]; then
    mkdir -p "$TAK_INSTALL_DIR" 2>/dev/null || err "failed to create TAK_INSTALL_DIR=$TAK_INSTALL_DIR"
    [[ -w "$TAK_INSTALL_DIR" ]] || err "TAK_INSTALL_DIR is not writable: $TAK_INSTALL_DIR"
    printf '%s' "$TAK_INSTALL_DIR"
    return
  fi

  for candidate in "$HOME/.local/bin" "$HOME/bin"; do
    if mkdir -p "$candidate" 2>/dev/null && [[ -w "$candidate" ]]; then
      printf '%s' "$candidate"
      return
    fi
  done

  err "failed to resolve a writable install directory; set TAK_INSTALL_DIR to override"
}

active_shell_rc() {
  local shell_name
  shell_name="$(basename -- "${SHELL:-}")"
  case "$shell_name" in
    zsh)
      printf '%s' "$HOME/.zshrc"
      ;;
    bash)
      printf '%s' "$HOME/.bashrc"
      ;;
    *)
      printf '%s' "$HOME/.profile"
      ;;
  esac
}

cargo_version_is_stable() {
  local version
  version="$(cargo -V 2>/dev/null || true)"
  [[ "$version" =~ ^cargo[[:space:]][0-9]+\.[0-9]+\.[0-9]+[[:space:]] ]]
}

cargo_supports_version_probe() {
  cargo -V >/dev/null 2>&1
}

resolve_cargo_cmd() {
  if cargo +stable -V >/dev/null 2>&1; then
    CARGO_CMD=(cargo +stable)
    return
  fi

  if cargo_version_is_stable; then
    CARGO_CMD=(cargo)
    return
  fi

  # Keep plain-cargo behavior for minimal shims that only implement build/metadata.
  if ! cargo_supports_version_probe; then
    CARGO_CMD=(cargo)
    return
  fi

  err "stable Rust toolchain is required for local source installs; install it with 'rustup toolchain install stable' or make stable your active toolchain"
}

cargo_run() {
  "${CARGO_CMD[@]}" "$@"
}

cargo_target_dir() {
  local metadata target_dir

  if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    printf '%s' "$CARGO_TARGET_DIR"
    return
  fi

  if metadata="$(cargo_run metadata --format-version 1 --no-deps 2>/dev/null)"; then
    target_dir="$(printf '%s' "$metadata" | tr -d '\n' | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
    target_dir="${target_dir//\\\//\/}"
    if [[ -n "$target_dir" ]]; then
      printf '%s' "$target_dir"
      return
    fi
  fi

  printf 'target'
}

ensure_path() {
  local dir="$1"
  local rc_file line

  case ":$PATH:" in
    *":$dir:"*)
      return
      ;;
  esac

  rc_file="$(active_shell_rc)"
  line="export PATH=\"$dir:\$PATH\""

  if [[ -f "$rc_file" ]] && grep -Fqx "$line" "$rc_file"; then
    :
  else
    printf '\n%s\n' "$line" >> "$rc_file"
  fi

  export PATH="$dir:$PATH"
  printf 'Added %s to PATH in %s\n' "$dir" "$rc_file"
  printf 'Reload your shell or run: source %s\n' "$rc_file"
}

main() {
  local root dir target_root tak_artifact takd_artifact

  [[ -n "${HOME:-}" ]] || err "HOME is required"

  root="$(repo_root)"
  cd "$root"
  resolve_cargo_cmd

  dir="$(install_dir)"

  printf 'Building tak and takd from source\n'
  cargo_run build --release --locked -p tak -p takd

  target_root="$(cargo_target_dir)"
  tak_artifact="$target_root/release/tak"
  takd_artifact="$target_root/release/takd"

  [[ -f "$tak_artifact" ]] || err "missing build artifact $tak_artifact"
  [[ -f "$takd_artifact" ]] || err "missing build artifact $takd_artifact"

  install -m 0755 "$tak_artifact" "$dir/tak"
  install -m 0755 "$takd_artifact" "$dir/takd"

  ensure_path "$dir"

  printf 'Installed tak and takd to %s\n' "$dir"
  "$dir/tak" --version || true
}

main "$@"
