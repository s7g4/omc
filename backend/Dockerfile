# syntax=docker/dockerfile:1
FROM rust:1-slim-bookworm AS builder
# curl + ca-certificates: utoipa-swagger-ui's build.rs shells out to `curl` at compile time to
# download the Swagger UI static assets — without it the build fails with a confusing panic.
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler pkg-config libssl-dev curl ca-certificates \
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
RUN cargo build --release -p backend

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/backend /app/backend
COPY backend/config /app/config
# Migrations are embedded into the binary at compile time via sqlx::migrate!, so the
# migrations/ directory doesn't need to be copied into the runtime image.

ENV HTTP_HOST=0.0.0.0
ENV GRPC_HOST=0.0.0.0
EXPOSE 8081 50051

CMD ["/app/backend"]
