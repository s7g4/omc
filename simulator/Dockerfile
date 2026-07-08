# syntax=docker/dockerfile:1
FROM rust:1-slim-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml backend/Cargo.toml
COPY simulator/Cargo.toml simulator/Cargo.toml
COPY backend backend
COPY simulator simulator
COPY proto proto
COPY .sqlx .sqlx

ENV SQLX_OFFLINE=true
RUN cargo build --release -p simulator

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/simulator /app/simulator
# simulator_config.json (satellite identity) is generated on first boot if absent — ephemeral
# per-container unless a volume is mounted over /app, which is fine for demo purposes.

CMD ["/app/simulator"]
