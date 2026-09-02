#!/usr/bin/env sh
set -eu

if [ "$#" -lt 1 ] || [ "$1" != "build" ]; then
  if [ "$#" -ge 1 ] && [ "$1" = "metadata" ]; then
    target_dir="${CARGO_TARGET_DIR:-${FAKE_METADATA_TARGET_DIR:-target}}"
    printf '{"target_directory":"%s"}\n' "${target_dir}"
    exit 0
  fi

  printf 'unexpected cargo invocation: %s\n' "$*" >&2
  exit 1
fi

case " $* " in
  *" --release "* ) ;;
  * ) printf 'missing --release in cargo invocation\n' >&2; exit 1 ;;
esac
case " $* " in
  *" --locked "* ) ;;
  * ) printf 'missing --locked in cargo invocation\n' >&2; exit 1 ;;
esac
case " $* " in
  *" -p tak "* ) ;;
  * ) printf 'missing -p tak in cargo invocation\n' >&2; exit 1 ;;
esac
case " $* " in
  *" -p takd "* ) ;;
  * ) printf 'missing -p takd in cargo invocation\n' >&2; exit 1 ;;
esac

target_dir="${CARGO_TARGET_DIR:-${FAKE_METADATA_TARGET_DIR:-target}}"
tag="${FAKE_BUILD_TAG:-dev}"

mkdir -p "${target_dir}/release"

cat > "${target_dir}/release/tak" <<EOF
#!/usr/bin/env sh
if [ "\${1:-}" = "--version" ]; then
  printf 'tak %s\n' "${tag}"
else
  printf 'tak %s\n' "${tag}"
fi
EOF

cat > "${target_dir}/release/takd" <<EOF
#!/usr/bin/env sh
printf 'takd %s\n' "${tag}"
EOF

chmod +x "${target_dir}/release/tak" "${target_dir}/release/takd"
