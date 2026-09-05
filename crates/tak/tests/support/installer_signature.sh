download_asset() {
  case "$1" in
    *.minisig)
      [ "$SIGNATURE_MODE" != missing ] || return 22
      ;;
  esac
  : > "$2"
}

minisign() {
  printf '%s\n' "$@" > "$SIGNATURE_TEST_ROOT/verified"
  [ "$SIGNATURE_MODE" = valid ]
}

tar() {
  : > "$SIGNATURE_TEST_ROOT/extracted"
  return 86
}

if [ "$SIGNATURE_MODE" = no_verifier ]; then
  unset -f minisign
  PATH=''
fi
