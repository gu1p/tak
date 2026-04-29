#!/usr/bin/env bash
set -euo pipefail

TAK_REPO="${TAK_REPO:-gu1p/tak}"
TAKD_INSTALL_DIR="${TAKD_INSTALL_DIR:-$HOME/.local/bin}"
TAKD_VERSION_INPUT="${TAKD_VERSION:-${TAK_VERSION:-}}"
TAKD_WAIT_TIMEOUT_SECS="${TAKD_WAIT_TIMEOUT_SECS:-120}"

err() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

download_asset() {
  local url="$1" out_file="$2"
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
    Linux) os='unknown-linux-musl' ;;
    Darwin) os='apple-darwin' ;;
    *) err "unsupported operating system: $os" ;;
  esac
  case "$arch" in
    x86_64|amd64) arch='x86_64' ;;
    arm64|aarch64) arch='aarch64' ;;
    *) err "unsupported architecture: $arch" ;;
  esac
  printf '%s-%s' "$arch" "$os"
}

resolve_tag() {
  if [[ -n "$TAKD_VERSION_INPUT" ]]; then
    [[ "$TAKD_VERSION_INPUT" == v* ]] && printf '%s' "$TAKD_VERSION_INPUT" || printf 'v%s' "$TAKD_VERSION_INPUT"
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

config_home() {
  printf '%s' "${XDG_CONFIG_HOME:-$HOME/.config}"
}

state_home() {
  printf '%s' "${XDG_STATE_HOME:-$HOME/.local/state}"
}

install_linux_service() {
  local takd_bin="$1" unit_dir unit_file state_root config_root
  unit_dir="$(config_home)/systemd/user"
  unit_file="$unit_dir/takd.service"
  config_root="$(config_home)/takd"
  state_root="$(state_home)/takd"
  mkdir -p "$unit_dir" "$state_root"
  cat >"$unit_file" <<EOF
[Unit]
Description=Tak execution agent
After=default.target

[Service]
ExecStart=${takd_bin} serve --config-root ${config_root} --state-root ${state_root}
Restart=always
RestartSec=2
WorkingDirectory=%h

[Install]
WantedBy=default.target
EOF
  if command -v systemctl >/dev/null 2>&1; then
    if systemctl --user daemon-reload >/dev/null 2>&1 \
      && systemctl --user enable takd.service >/dev/null 2>&1 \
      && systemctl --user restart takd.service >/dev/null 2>&1; then
      if command -v loginctl >/dev/null 2>&1; then
        loginctl enable-linger "${USER:-$(id -un)}" >/dev/null 2>&1 || true
      fi
      return 0
    fi
  fi
  return 1
}

install_macos_service() {
  local takd_bin="$1" plist_dir plist_file state_root config_root uid
  plist_dir="$HOME/Library/LaunchAgents"
  plist_file="$plist_dir/dev.tak.takd.plist"
  config_root="$(config_home)/takd"
  state_root="$(state_home)/takd"
  uid="$(id -u)"
  mkdir -p "$plist_dir" "$state_root"
  cat >"$plist_file" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>dev.tak.takd</string>
<key>ProgramArguments</key><array><string>${takd_bin}</string><string>serve</string><string>--config-root</string><string>${config_root}</string><string>--state-root</string><string>${state_root}</string></array>
<key>RunAtLoad</key><true/>
<key>KeepAlive</key><true/>
</dict></plist>
EOF
  launchctl bootout "gui/${uid}" "$plist_file" >/dev/null 2>&1 || true
  launchctl bootstrap "gui/${uid}" "$plist_file"
  launchctl kickstart -k "gui/${uid}/dev.tak.takd"
}

install_service() {
  local takd_bin="$1"
  case "$(uname -s)" in
    Linux) install_linux_service "$takd_bin" ;;
    Darwin) install_macos_service "$takd_bin" ;;
    *) err "per-user takd service install is only supported on Linux and macOS" ;;
  esac
}

agent_config_path() {
  printf '%s/takd/agent.toml' "$(config_home)"
}

agent_state_path() {
  printf '%s/takd' "$(state_home)"
}

main() {
  local target tag archive_name archive_url temp_dir archive_path takd_bin base_url token_output token_status
  local -a init_args
  target="$(detect_target)"
  tag="$(resolve_tag)"
  archive_name="tak-${tag}-${target}.tar.gz"
  archive_url="https://github.com/${TAK_REPO}/releases/download/${tag}/${archive_name}"
  temp_dir="$(mktemp -d)"
  trap "rm -rf -- '$temp_dir'" EXIT
  archive_path="$temp_dir/$archive_name"

  printf 'Downloading %s\n' "$archive_url"
  download_asset "$archive_url" "$archive_path" || err "failed to download release artifact ${archive_name}"
  tar -xzf "$archive_path" -C "$temp_dir"
  [[ -f "$temp_dir/takd" ]] || err "archive missing takd binary"

  mkdir -p "$TAKD_INSTALL_DIR"
  takd_bin="$TAKD_INSTALL_DIR/takd"
  install -m 0755 "$temp_dir/takd" "$takd_bin"
  if [[ ! -f "$(agent_config_path)" ]]; then
    init_args=(init)
    [[ -n "${TAKD_NODE_ID:-}" ]] && init_args+=(--node-id "$TAKD_NODE_ID")
    [[ -n "${TAKD_DISPLAY_NAME:-}" ]] && init_args+=(--display-name "$TAKD_DISPLAY_NAME")
    [[ -n "${TAKD_TRANSPORT:-}" ]] && init_args+=(--transport "$TAKD_TRANSPORT")
    [[ -n "${TAKD_BASE_URL:-}" ]] && init_args+=(--base-url "$TAKD_BASE_URL")
    if [[ -n "${TAKD_POOLS:-}" ]]; then
      IFS=',' read -r -a values <<<"$TAKD_POOLS"
      for value in "${values[@]}"; do
        value="${value#"${value%%[![:space:]]*}"}"
        value="${value%"${value##*[![:space:]]}"}"
        [[ -n "$value" ]] && init_args+=(--pool "$value")
      done
    fi
    if [[ -n "${TAKD_TAGS:-}" ]]; then
      IFS=',' read -r -a values <<<"$TAKD_TAGS"
      for value in "${values[@]}"; do
        value="${value#"${value%%[![:space:]]*}"}"
        value="${value%"${value##*[![:space:]]}"}"
        [[ -n "$value" ]] && init_args+=(--tag "$value")
      done
    fi
    if [[ -n "${TAKD_CAPABILITIES:-}" ]]; then
      IFS=',' read -r -a values <<<"$TAKD_CAPABILITIES"
      for value in "${values[@]}"; do
        value="${value#"${value%%[![:space:]]*}"}"
        value="${value%"${value##*[![:space:]]}"}"
        [[ -n "$value" ]] && init_args+=(--capability "$value")
      done
    fi
    "$takd_bin" "${init_args[@]}"
  fi
  if ! install_service "$takd_bin"; then
    printf 'takd installed, but automatic service startup is unavailable on this host.\n'
    printf 'start manually:\n  %s serve --config-root %s --state-root %s\n' "$takd_bin" "$(config_home)/takd" "$(agent_state_path)"
    printf 'then fetch a token with:\n  %s token show --state-root %s --wait --timeout-secs %s\n' "$takd_bin" "$(agent_state_path)" "$TAKD_WAIT_TIMEOUT_SECS"
    exit 0
  fi

  set +e
  token_output="$("$takd_bin" token show --state-root "$(agent_state_path)" --wait --timeout-secs "$TAKD_WAIT_TIMEOUT_SECS" --qr 2>&1)"
  token_status=$?
  set -e
  if [[ "$token_status" -ne 0 ]]; then
    printf '%s\n' "$token_output" >&2
    printf 'recent takd logs:\n' >&2
    "$takd_bin" logs --state-root "$(agent_state_path)" --lines 100 >&2 || true
    exit "$token_status"
  fi
  base_url="$("$takd_bin" status --config-root "$(config_home)/takd" --state-root "$(agent_state_path)" | sed -n 's/^base_url: //p' | head -n1)"
  printf 'takd ready at %s\n' "${base_url:-unknown}"
  printf '%s\n' "$token_output"
}

main "$@"
