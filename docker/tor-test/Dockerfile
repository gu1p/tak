# Minimal runtime image for the two-node real-Tor integration test.
#
# Holds BOTH binaries; node A runs only `takd` (a remote node), node B runs
# `takd` (local bridge/daemon) + `tak` (client). MOCK_CONTAINER=true makes the
# remote worker simulate container execution so takd can run inside a container
# without a nested Docker/Podman runtime.
#
# Binaries are built on the host (glibc <= 2.39, no OpenSSL, arti=rustls, no
# system tor) and copied in, so debian:trixie-slim (glibc 2.41) is sufficient.
FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY takd /usr/local/bin/takd
COPY tak  /usr/local/bin/tak

# Simulate container execution everywhere (takd cannot run nested containers).
ENV MOCK_CONTAINER=true
# Surface Tor connection lifecycle signal; mute per-status-event bootstrap noise.
ENV RUST_LOG=warn,takd=info,takd::daemon::peer_manager=debug,takd::daemon::protocol::broker=debug,tak_exec=info
ENV HOME=/root

WORKDIR /work
CMD ["takd", "--help"]
