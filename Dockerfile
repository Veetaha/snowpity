# Possible values: debug or release
ARG RUST_BUILD_MODE="debug"

FROM lukemathwalker/cargo-chef:0.1.35-rust-1.61.0 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Conditional block for compilation in either debug or release mode
FROM chef as rust_debug_builder
ENV RUST_RELEASE_FLAG=""

FROM chef as rust_release_builder
ENV RUST_RELEASE_FLAG="--release"

FROM rust_${RUST_BUILD_MODE}_builder as builder
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook ${RUST_RELEASE_FLAG} --recipe-path recipe.json

# Build application
COPY . .

RUN cargo build ${RUST_RELEASE_FLAG} -p veebot-telegram --bin veebot-telegram

# We do not need the Rust toolchain to run the binary!
FROM debian:11-slim AS runtime

# Must be defined again for this stage; uses global default value if it isn't set
ARG RUST_BUILD_MODE

WORKDIR /app

COPY --from=builder /app/target/${RUST_BUILD_MODE}/veebot-telegram /usr/local/bin

# Not an expert in SSL, but this seems to be required for all SSL-encrypted communication.
# Thanks to this guy for help:
# https://github.com/debuerreotype/docker-debian-artifacts/issues/15#issuecomment-634423712
RUN apt-get update
RUN apt-get install -y --no-install-recommends ca-certificates \
    libopus0

ENTRYPOINT ["/usr/local/bin/veebot-telegram"]
