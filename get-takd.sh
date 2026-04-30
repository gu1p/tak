#!/usr/bin/env bash
set -euo pipefail

TAK_REPO="${TAK_REPO:-gu1p/tak}"
TAKD_INSTALL_DIR="${TAKD_INSTALL_DIR:-$HOME/.local/bin}"
TAKD_VERSION_INPUT="${TAKD_VERSION:-${TAK_VERSION:-}}"
TAKD_WAIT_TIMEOUT_SECS="${TAKD_WAIT_TIMEOUT_SECS:-360}"
TAKD_WAIT_POLL_SECS="${TAKD_WAIT_POLL_SECS:-5}"
log_tail_pid=""
last_readiness_highlight=""

err() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

installer_verbose() {
  case "${TAKD_INSTALLER_VERBOSE:-}" in
    1|true|TRUE|yes|YES|on|ON) return 0 ;;
    *) return 1 ;;
  esac
}

supports_color() {
  [[ -t 1 ]] && [[ -z "${NO_COLOR:-}" ]]
}

tag_color_code() {
  case "$1" in
    download) printf '36' ;;
    install|ready) printf '32' ;;
    service) printf '34' ;;
    tor) printf '33' ;;
    *) printf '0' ;;
  esac
}

highlight() {
  local tag="$1" message="$2"
  if supports_color; then
    printf '\033[%sm[%s]\033[0m %s\n' "$(tag_color_code "$tag")" "$tag" "$message"
  else
    printf '[%s] %s\n' "$tag" "$message"
  fi
}

download_asset() {
  local url="$1" out_file="$2"
  curl -fL --progress-bar -o "$out_file" "$url"
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

default_service_rust_log() {
  printf '%s' "${TAKD_SERVICE_RUST_LOG:-trace}"
}

default_service_backtrace() {
  printf '%s' "${TAKD_SERVICE_RUST_BACKTRACE:-1}"
}

linux_service_name() {
  printf 'takd.service'
}

macos_service_label() {
  printf 'dev.tak.takd'
}

stop_linux_service() {
  command -v systemctl >/dev/null 2>&1 || return 0
  systemctl --user stop "$(linux_service_name)" >/dev/null 2>&1 || true
}

start_linux_service() {
  command -v systemctl >/dev/null 2>&1 || return 1
  systemctl --user daemon-reload >/dev/null 2>&1 \
    && systemctl --user enable "$(linux_service_name)" >/dev/null 2>&1 \
    && systemctl --user restart "$(linux_service_name)" >/dev/null 2>&1
}

stop_macos_service() {
  local uid
  command -v launchctl >/dev/null 2>&1 || return 0
  uid="$(id -u)"
  launchctl bootout "gui/${uid}/$(macos_service_label)" >/dev/null 2>&1 || true
}

start_macos_service() {
  local plist_file="$1" uid
  uid="$(id -u)"
  launchctl bootstrap "gui/${uid}" "$plist_file"
  launchctl kickstart -k "gui/${uid}/$(macos_service_label)"
}

reset_rebuildable_arti_state() {
  local arti_root
  arti_root="$(agent_state_path)/arti"
  rm -rf -- \
    "$arti_root/cache" \
    "$arti_root/state/state" \
    "$arti_root/state/hss" \
    "$arti_root/state/hss_iptreplay" \
    "$arti_root/state/pt_state"
}

install_linux_service() {
  local takd_bin="$1" unit_dir unit_file state_root config_root service_rust_log service_backtrace
  unit_dir="$(config_home)/systemd/user"
  unit_file="$unit_dir/takd.service"
  config_root="$(config_home)/takd"
  state_root="$(state_home)/takd"
  service_rust_log="$(default_service_rust_log)"
  service_backtrace="$(default_service_backtrace)"
  mkdir -p "$unit_dir" "$state_root"
  cat >"$unit_file" <<EOF
[Unit]
Description=Tak execution agent
After=default.target

[Service]
Environment=RUST_LOG=${service_rust_log}
Environment=RUST_BACKTRACE=${service_backtrace}
ExecStart=${takd_bin} serve --config-root ${config_root} --state-root ${state_root}
Restart=always
RestartSec=2
WorkingDirectory=%h

[Install]
WantedBy=default.target
EOF
  stop_linux_service
  reset_rebuildable_arti_state
  if start_linux_service; then
    if command -v loginctl >/dev/null 2>&1; then
      loginctl enable-linger "${USER:-$(id -un)}" >/dev/null 2>&1 || true
    fi
    return 0
  fi
  return 1
}

install_macos_service() {
  local takd_bin="$1" plist_dir plist_file state_root config_root service_rust_log service_backtrace
  plist_dir="$HOME/Library/LaunchAgents"
  plist_file="$plist_dir/dev.tak.takd.plist"
  config_root="$(config_home)/takd"
  state_root="$(state_home)/takd"
  service_rust_log="$(default_service_rust_log)"
  service_backtrace="$(default_service_backtrace)"
  mkdir -p "$plist_dir" "$state_root"
  cat >"$plist_file" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>dev.tak.takd</string>
<key>ProgramArguments</key><array><string>${takd_bin}</string><string>serve</string><string>--config-root</string><string>${config_root}</string><string>--state-root</string><string>${state_root}</string></array>
<key>EnvironmentVariables</key><dict><key>RUST_LOG</key><string>${service_rust_log}</string><key>RUST_BACKTRACE</key><string>${service_backtrace}</string></dict>
<key>RunAtLoad</key><true/>
<key>KeepAlive</key><true/>
</dict></plist>
EOF
  stop_macos_service
  reset_rebuildable_arti_state
  start_macos_service "$plist_file"
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

service_log_path() {
  printf '%s/service.log' "$(agent_state_path)"
}

transport_health_path() {
  printf '%s/transport-health.toml' "$(agent_state_path)"
}

start_log_tail() {
  local log_path="$1"
  printf 'service_log: %s\n' "$log_path" >&2
  printf 'streaming takd service log with tail -F\n' >&2
  tail -n 0 -F "$log_path" >&2 &
  log_tail_pid=$!
}

stop_log_tail() {
  if [[ -n "${log_tail_pid:-}" ]]; then
    kill "$log_tail_pid" >/dev/null 2>&1 || true
    wait "$log_tail_pid" >/dev/null 2>&1 || true
    log_tail_pid=""
  fi
}

print_readiness_snapshot() {
  local takd_bin="$1" status_output health_path log_path
  health_path="$(transport_health_path)"
  log_path="$(service_log_path)"
  printf '\n--- takd readiness snapshot (%s) ---\n' "$(date -Is 2>/dev/null || date)" >&2
  printf 'config_root: %s\n' "$(config_home)/takd" >&2
  printf 'state_root: %s\n' "$(agent_state_path)" >&2
  printf 'service_log: %s\n' "$log_path" >&2
  printf 'transport_health: %s\n' "$health_path" >&2
  if status_output="$("$takd_bin" status --config-root "$(config_home)/takd" --state-root "$(agent_state_path)" 2>&1)"; then
    printf '%s\n' "$status_output" >&2
  else
    printf 'takd status failed:\n%s\n' "$status_output" >&2
  fi
  if [[ -f "$health_path" ]]; then
    printf 'transport-health.toml:\n' >&2
    sed 's/^/  /' "$health_path" >&2
  else
    printf 'transport-health.toml: not present yet\n' >&2
  fi
  printf 'recent takd logs:\n' >&2
  "$takd_bin" logs --state-root "$(agent_state_path)" --lines 80 >&2 || true
  printf -- '--- end takd readiness snapshot ---\n\n' >&2
}

latest_transport_detail() {
  local takd_bin="$1" health_path status_output detail
  health_path="$(transport_health_path)"
  if [[ -f "$health_path" ]]; then
    detail="$(sed -n 's/^detail = "\(.*\)"$/\1/p' "$health_path" | tail -n1)"
    if [[ -n "$detail" ]]; then
      printf '%s' "$detail"
      return
    fi
  fi
  if status_output="$("$takd_bin" status --config-root "$(config_home)/takd" --state-root "$(agent_state_path)" 2>/dev/null)"; then
    detail="$(printf '%s\n' "$status_output" | sed -n 's/^transport_detail: //p' | head -n1)"
    [[ -n "$detail" ]] && printf '%s' "$detail"
  fi
}

print_readiness_highlight() {
  local takd_bin="$1" detail
  detail="$(latest_transport_detail "$takd_bin" || true)"
  [[ -n "$detail" ]] || detail="takd is still preparing its Tor service"
  if [[ "$detail" != "$last_readiness_highlight" ]]; then
    highlight tor "pending: $detail"
    last_readiness_highlight="$detail"
  fi
}

wait_for_token_with_progress() {
  local takd_bin="$1" deadline remaining attempt_timeout attempt_status attempt_output attempt poll_secs
  token_output=""
  attempt=1
  poll_secs="$TAKD_WAIT_POLL_SECS"
  if ! [[ "$poll_secs" =~ ^[0-9]+$ ]] || [[ "$poll_secs" -lt 1 ]]; then
    poll_secs=5
  fi
  deadline=$((SECONDS + TAKD_WAIT_TIMEOUT_SECS))
  while true; do
    remaining=$((deadline - SECONDS))
    if [[ "$remaining" -le 0 ]]; then
      printf 'takd Tor readiness timed out after %ss\n' "$TAKD_WAIT_TIMEOUT_SECS" >&2
      return 1
    fi
    attempt_timeout="$poll_secs"
    if [[ "$attempt_timeout" -gt "$remaining" ]]; then
      attempt_timeout="$remaining"
    fi
    if installer_verbose; then
      printf 'takd readiness token attempt %s (timeout %ss, remaining %ss)\n' "$attempt" "$attempt_timeout" "$remaining" >&2
    fi
    set +e
    attempt_output="$("$takd_bin" token show --state-root "$(agent_state_path)" --wait --timeout-secs "$attempt_timeout" --qr 2>&1)"
    attempt_status=$?
    set -e
    token_output="$attempt_output"
    if [[ "$attempt_status" -eq 0 ]]; then
      return 0
    fi
    if installer_verbose; then
      printf 'takd token is not ready yet:\n%s\n' "$attempt_output" >&2
      print_readiness_snapshot "$takd_bin"
    else
      print_readiness_highlight "$takd_bin"
    fi
    remaining=$((deadline - SECONDS))
    if [[ "$remaining" -le 0 ]]; then
      return "$attempt_status"
    fi
    sleep 1
    attempt=$((attempt + 1))
  done
}

main() {
  local target tag archive_name archive_url temp_dir archive_path takd_bin base_url token_output token_status
  local -a init_args
  target="$(detect_target)"
  tag="$(resolve_tag)"
  archive_name="tak-${tag}-${target}.tar.gz"
  archive_url="https://github.com/${TAK_REPO}/releases/download/${tag}/${archive_name}"
  temp_dir="$(mktemp -d)"
  trap "stop_log_tail; rm -rf -- '$temp_dir'" EXIT
  archive_path="$temp_dir/$archive_name"

  highlight download "Fetching takd release ${archive_name}"
  download_asset "$archive_url" "$archive_path" || err "failed to download release artifact ${archive_name}"
  tar -xzf "$archive_path" -C "$temp_dir"
  [[ -f "$temp_dir/takd" ]] || err "archive missing takd binary"

  mkdir -p "$TAKD_INSTALL_DIR"
  takd_bin="$TAKD_INSTALL_DIR/takd"
  install -m 0755 "$temp_dir/takd" "$takd_bin"
  highlight install "Installed takd to ${takd_bin}"
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
    printf 'start manually:\n  RUST_LOG=%s RUST_BACKTRACE=%s %s serve --config-root %s --state-root %s\n' "$(default_service_rust_log)" "$(default_service_backtrace)" "$takd_bin" "$(config_home)/takd" "$(agent_state_path)"
    printf 'then fetch a token with:\n  %s token show --state-root %s --wait --timeout-secs %s\n' "$takd_bin" "$(agent_state_path)" "$TAKD_WAIT_TIMEOUT_SECS"
    exit 0
  fi

  highlight service "Started takd user service"
  highlight tor "Waiting for readiness (timeout ${TAKD_WAIT_TIMEOUT_SECS}s)"
  printf 'Full logs: %s\n' "$(service_log_path)"
  printf 'View all logs: takd logs --all\n'
  if installer_verbose; then
    printf 'transport_health: %s\n' "$(transport_health_path)" >&2
    start_log_tail "$(service_log_path)"
    print_readiness_snapshot "$takd_bin"
  fi
  if wait_for_token_with_progress "$takd_bin"; then
    stop_log_tail
  else
    token_status=$?
    stop_log_tail
    printf '%s\n' "$token_output" >&2
    print_readiness_snapshot "$takd_bin"
    printf 'recent takd logs:\n' >&2
    "$takd_bin" logs --state-root "$(agent_state_path)" --lines 100 >&2 || true
    exit "$token_status"
  fi
  base_url="$("$takd_bin" status --config-root "$(config_home)/takd" --state-root "$(agent_state_path)" | sed -n 's/^base_url: //p' | head -n1)"
  highlight ready "takd ready at ${base_url:-unknown}"
  printf '%s\n' "$token_output"
}

main "$@"
