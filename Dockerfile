# syntax = docker/dockerfile:1.2

# Possible values: debug or release
ARG RUST_BUILD_MODE="debug"

FROM rust:1.65.0-bullseye as build_base

ENV CARGO_NET_RETRY=50
ENV CARGO_TERM_COLOR=always

WORKDIR /app

# Conditional block for compilation in either debug or release mode
FROM build_base as build_debug
ENV RUST_RELEASE_FLAG=""

FROM build_base as build_release
ENV RUST_RELEASE_FLAG="--release"

FROM build_${RUST_BUILD_MODE} as build

ARG RUST_BUILD_MODE

# Build application
COPY . .

RUN --mount=type=cache,sharing=private,target=/usr/local/cargo/git \
    --mount=type=cache,sharing=private,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=private,target=/app/target \
    cargo build ${RUST_RELEASE_FLAG} -p veebot-telegram --bin veebot-telegram && \
    # The buildkit's cache dir (`target`) isn't part of docker layers, so we need to
    # copy the binary out of that dir into somewhere where it will be part of the layer.
    cp /app/target/${RUST_BUILD_MODE}/veebot-telegram /usr/bin/

# We do not need the Rust toolchain to run the binary!
FROM debian:11-slim AS runtime

# Must be defined again for this stage; uses global default value if it isn't set
ARG RUST_BUILD_MODE

WORKDIR /app

COPY scripts/install-runtime-deps.sh /app

# The apt caching setup is inspired by https://vsupalov.com/buildkit-cache-mount-dockerfile/
#
# Debian docker image contains configurations to delete cached files after a successful install
# Using a cache mount would not make sense with this configuration in place, as the files would
# be deleted during the install step.
RUN rm -f /etc/apt/apt.conf.d/docker-clean

RUN --mount=type=cache,target=/var/cache/apt /app/install-runtime-deps.sh

COPY --from=build /usr/bin/veebot-telegram /usr/bin

ENTRYPOINT ["/usr/bin/veebot-telegram"]
