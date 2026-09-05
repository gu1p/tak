use std::path::Path;

use super::archive::write_executable;

pub(super) fn install(bin: &Path) {
    write_executable(
        &bin.join("uname"),
        "#!/bin/sh\n[ \"${1:-}\" = -m ] && echo x86_64 || echo Linux\n",
    );
    write_executable(&bin.join("curl"), curl());
    write_executable(&bin.join("systemctl"), "#!/bin/sh\nexit 1\n");
    write_executable(&bin.join("cargo"), cargo());
    write_executable(
        &bin.join("minisign"),
        include_str!("../installer_minisign.sh"),
    );
}

fn curl() -> &'static str {
    r#"#!/bin/sh
set -eu
out=''
previous=''
for argument in "$@"; do
  if [ "$previous" = -o ]; then out="$argument"; fi
  previous="$argument"
done
[ -n "$out" ] || exit 2
cp "$FAKE_RELEASE_ARCHIVE" "$out"
"#
}

fn cargo() -> &'static str {
    r#"#!/bin/sh
set -eu
if [ "${1:-}" = +stable ]; then shift; fi
case "${1:-}" in
  -V) echo 'cargo 1.90.0 (fixture)';;
  build) ;;
  metadata) printf '{"target_directory":"%s"}\n' "$CARGO_TARGET_DIR";;
  *) echo "unexpected cargo invocation: $*" >&2; exit 2;;
esac
"#
}
