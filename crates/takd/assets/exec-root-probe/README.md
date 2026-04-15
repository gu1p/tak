## Exec-Root Probe Helpers

This directory holds the statically linked Linux helpers embedded into the
exec-root probe image.

- `busybox-x86_64`: existing x86_64 helper binary.
- `busybox-aarch64`: minimal `aarch64` probe helper used when the container
  daemon reports `arm64`/`aarch64`.

The `busybox-aarch64` helper was built locally from a tiny syscall-only entry
point with:

```sh
clang --target=aarch64-linux-musl -c /tmp/probe_aarch64.S -o /tmp/probe_aarch64.o
~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld \
  -flavor gnu -m aarch64linux -static -e _start -o /tmp/probe_aarch64 /tmp/probe_aarch64.o
/usr/lib/llvm-20/bin/llvm-strip /tmp/probe_aarch64
```
