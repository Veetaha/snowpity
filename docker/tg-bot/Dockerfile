# syntax = docker/dockerfile:1.2


# We could do cross-compilation by specifying `FROM --platform=$BUILDPLATFORM` here
# and then using `cargo-zigbuild`. However, we are not doing this because our
# transitive dependency `audiopus_sys` depends on a C library, and it's a huge
# pain to cross-compile C libraries. So we are just using the emulation provided
# by docker BuildKit with QEMU to do the build. The build time in this case is
# drastically increased, but it's not a big deal for us at the moment.
#
# See a detailed cross-compilation guide here:
# https://www.docker.com/blog/faster-multi-platform-builds-dockerfile-cross-compilation-guide/
FROM rust:1.72.0-bookworm as build

# Possible values: debug or release
ARG RUST_BUILD_MODE="debug"
ENV CARGO_NET_RETRY=50
ENV CARGO_TERM_COLOR=always

WORKDIR /app

# Don't clean the apt cache (see the reason on the same command in the runtime stage)
RUN rm -f /etc/apt/apt.conf.d/docker-clean

RUN --mount=target=scripts/download,source=scripts/download \
    --mount=target=docker/tg-bot,source=docker/tg-bot \
    --mount=type=cache,id=build-apt,target=/var/cache/apt \
    docker/tg-bot/install-build-deps.sh

ENV PATH="/usr/local/go/bin:$PATH"

RUN --mount=target=. \
    --mount=type=cache,id=build-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=build-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=build-target,target=/target \
    docker/tg-bot/build-rust.sh

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# The apt caching setup is inspired by https://vsupalov.com/buildkit-cache-mount-dockerfile/
#
# Debian docker image contains configurations to delete cached files after a successful install
# Using a cache mount would not make sense with this configuration in place, as the files would
# be deleted during the install step.
RUN rm -f /etc/apt/apt.conf.d/docker-clean

RUN --mount=target=scripts/download,source=scripts/download \
    --mount=target=docker/tg-bot,source=docker/tg-bot \
    --mount=type=cache,id=runtime-apt,target=/var/cache/apt \
    docker/tg-bot/install-runtime-deps.sh

COPY --from=build /usr/bin/snowpity-tg /usr/bin

ENTRYPOINT ["/usr/bin/snowpity-tg"]
