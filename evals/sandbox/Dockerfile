FROM rust:1-bookworm
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo
RUN apt-get update && apt-get install -y --no-install-recommends cmake pkg-config libssl-dev clang python3 time && rm -rf /var/lib/apt/lists/*
RUN rustup toolchain install nightly-2026-10-01 --profile minimal -c miri -c rust-src
RUN XARGO_CACHE=x MIRI_SYSROOT=/opt/miri-sysroot cargo +nightly-2026-10-01 miri setup
RUN chmod -R a+rX /usr/local/rustup /usr/local/cargo /opt/miri-sysroot && useradd -m -u 1000 audit && mkdir /target && chown audit:audit /target
USER audit
ENV RUSTUP_AUTO_INSTALL=0 RUSTUP_TOOLCHAIN= MIRI_SYSROOT=/opt/miri-sysroot
WORKDIR /work
