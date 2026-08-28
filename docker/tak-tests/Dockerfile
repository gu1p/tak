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

ARG AXIOM_VERSION=0.1.19
RUN curl -fsSL https://raw.githubusercontent.com/gu1p/axiom/main/install.sh \
        -o /tmp/axiom-install.sh \
    && env AXIOM_VERSION="${AXIOM_VERSION}" \
        AXIOM_NO_MODIFY_PATH=1 \
        AXIOM_INSTALL_DIR=/usr/local/cargo/bin \
        sh /tmp/axiom-install.sh \
    && rm /tmp/axiom-install.sh

ENV TAK_LIVE_TOR_TEST_TIMEOUT_SECS=420
ENV AXIOM_NO_MODIFY_PATH=1

WORKDIR /workspace
