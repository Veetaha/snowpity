FROM lukemathwalker/cargo-chef:0.1.35-rust-1.61.0 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .

RUN cargo build --release -p veebot-telegram --bin veebot-telegram

# We do not need the Rust toolchain to run the binary!
FROM debian:11-slim AS runtime

WORKDIR /app

COPY --from=builder /app/target/release/veebot-telegram /usr/local/bin

# Not an expert in SSL, but this seems to be required for all SSL-encrypted communication.
# Thanks to this guy for help:
# https://github.com/debuerreotype/docker-debian-artifacts/issues/15#issuecomment-634423712
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates

ENTRYPOINT ["/usr/local/bin/veebot-telegram"]
