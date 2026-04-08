#!/usr/bin/env bash
set -euo pipefail

TAK_REPO="${TAK_REPO:-gu1p/tak}"
TAK_INSTALL_DIR="${TAK_INSTALL_DIR:-$HOME/.local/bin}"
TAK_VERSION_INPUT="${TAK_VERSION:-}"
GITHUB_TOKEN_VALUE="${GH_TOKEN:-${GITHUB_TOKEN:-}}"

err() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

api_get() {
  local url="$1"
  if [[ -n "$GITHUB_TOKEN_VALUE" ]]; then
    curl -fsSL -H "Authorization: Bearer $GITHUB_TOKEN_VALUE" -H 'Accept: application/vnd.github+json' "$url"
  else
    curl -fsSL -H 'Accept: application/vnd.github+json' "$url"
  fi
}

download_asset() {
  local url="$1"
  local out_file="$2"
  if [[ -n "$GITHUB_TOKEN_VALUE" ]]; then
    curl -fsSL -H "Authorization: Bearer $GITHUB_TOKEN_VALUE" -o "$out_file" "$url"
  else
    curl -fsSL -o "$out_file" "$url"
  fi
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

  local latest_json tag
  latest_json="$(api_get "https://api.github.com/repos/${TAK_REPO}/releases/latest")" || {
    err "failed to resolve latest release for ${TAK_REPO}; if the repo is private set GH_TOKEN or GITHUB_TOKEN"
  }

  tag="$(printf '%s' "$latest_json" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"
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
    err "failed to download release artifact ${archive_name}; verify tag exists and auth token is set for private repos"
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
