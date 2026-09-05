command() {
  if [ "$1" = -v ] && [ "$2" = minisign ]; then
    return 1
  fi
  builtin command "$@"
}

minisign() {
  exit 87
}

download_asset() {
  printf '%s\n' "$1" >> "$INSTALLER_TEST_ROOT/downloads"
  [ "$DOWNLOAD_MODE" != missing ] || return 22
  : > "$2"
}

tar() {
  : > "$INSTALLER_TEST_ROOT/extracted"
  return 86
}
