#!/usr/bin/env bash
set -euo pipefail

TAK_REPO="${TAK_REPO:-gu1p/tak}"
TAK_INSTALL_DIR="${TAK_INSTALL_DIR:-$HOME/.local/bin}"
TAK_VERSION_INPUT="${TAK_VERSION:-}"

err() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

download_asset() {
  local url="$1"
  local out_file="$2"
  curl -fsSL -o "$out_file" "$url"
}

resolve_latest_release_url() {
  curl -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/${TAK_REPO}/releases/latest"
}

detect_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)
      os='unknown-linux-musl'
      ;;
    Darwin)
      os='apple-darwin'
      ;;
    *)
      err "unsupported operating system: $os"
      ;;
  esac

  case "$arch" in
    x86_64|amd64)
      arch='x86_64'
      ;;
    arm64|aarch64)
      arch='aarch64'
      ;;
    *)
      err "unsupported architecture: $arch"
      ;;
  esac

  printf '%s-%s' "$arch" "$os"
}

resolve_tag() {
  if [[ -n "$TAK_VERSION_INPUT" ]]; then
    if [[ "$TAK_VERSION_INPUT" == v* ]]; then
      printf '%s' "$TAK_VERSION_INPUT"
    else
      printf 'v%s' "$TAK_VERSION_INPUT"
    fi
    return
  fi

  local latest_url tag
  latest_url="$(resolve_latest_release_url)" || err "failed to resolve latest release for ${TAK_REPO}"
  case "$latest_url" in
    "https://github.com/${TAK_REPO}/releases/tag/"*)
      ;;
    *)
      err "could not parse latest release tag"
      ;;
  esac

  tag="${latest_url##*/}"
  tag="${tag%%\?*}"
  tag="${tag%/}"
  [[ -n "$tag" ]] || err "could not parse latest release tag"
  printf '%s' "$tag"
}

main() {
  local target tag archive_name archive_url temp_dir archive_path

  target="$(detect_target)"
  tag="$(resolve_tag)"

  archive_name="tak-${tag}-${target}.tar.gz"
  archive_url="https://github.com/${TAK_REPO}/releases/download/${tag}/${archive_name}"

  temp_dir="$(mktemp -d)"
  trap "rm -rf -- '$temp_dir'" EXIT
  archive_path="$temp_dir/$archive_name"

  printf 'Downloading %s\n' "$archive_url"
  download_asset "$archive_url" "$archive_path" || {
    err "failed to download release artifact ${archive_name}; verify the tag exists"
  }

  tar -xzf "$archive_path" -C "$temp_dir"
  [[ -f "$temp_dir/tak" ]] || err "archive missing tak binary"
  [[ -f "$temp_dir/takd" ]] || err "archive missing takd binary"

  mkdir -p "$TAK_INSTALL_DIR"
  install -m 0755 "$temp_dir/tak" "$TAK_INSTALL_DIR/tak"
  install -m 0755 "$temp_dir/takd" "$TAK_INSTALL_DIR/takd"

  printf 'Installed tak and takd to %s\n' "$TAK_INSTALL_DIR"
  "$TAK_INSTALL_DIR/tak" --version || true

  case ":$PATH:" in
    *":$TAK_INSTALL_DIR:"*)
      ;;
    *)
      printf 'Add this to your shell profile:\n'
      printf '  export PATH="%s:$PATH"\n' "$TAK_INSTALL_DIR"
      ;;
  esac
}

main "$@"
