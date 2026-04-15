FROM rust:bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        bash \
        ca-certificates \
        clang \
        curl \
        git \
        libclang-dev \
        make \
        pkg-config \
        ripgrep \
    && rm -rf /var/lib/apt/lists/*

RUN git config --system --add safe.directory '*'

RUN rustup toolchain install nightly-2026-03-16 --profile minimal --component rustfmt --component clippy \
    && rustup default nightly-2026-03-16

ENV TAK_LIVE_TOR_TEST_TIMEOUT_SECS=420

WORKDIR /workspace
