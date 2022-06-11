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
COPY Cargo.lock Cargo.toml veebot-telegram/
RUN cargo build --release --bin app

# We do not need the Rust toolchain to run the binary!
FROM debian:11-slim AS runtime

WORKDIR /app

COPY --from=builder /app/target/release/veebot-telegram /usr/local/bin

ENTRYPOINT ["/usr/local/bin/veebot-telegram"]
