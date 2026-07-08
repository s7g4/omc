---
title: Open Mission Control
layout: default
---

# Open Mission Control

A real-time satellite ground-control platform: a Rust/Axum backend that ingests telemetry over
gRPC, stores it in a TimescaleDB hypertable, fans it out over Redis pub/sub and NATS JetStream,
and serves it to a Next.js mission-ops dashboard over WebSockets.

![Demo](demo.gif)

[View the source on GitHub](https://github.com/s7g4/omc)

## Docs

- [Demo Walkthrough](DEMO.html) — how to run it, what to click, what to expect
- [Case Study](CASE_STUDY.html) — the engineering decisions behind the stack
- Architecture Decision Records:
  - [ADR-001: Axum](adr/adr-001-axum.html)
  - [ADR-002: NATS JetStream](adr/adr-002-nats-jetstream.html)
  - [ADR-003: TimescaleDB](adr/adr-003-timescaledb.html)
  - [ADR-004: gRPC](adr/adr-004-grpc.html)

## Highlights

- Dual gRPC + HTTP telemetry ingestion into a TimescaleDB hypertable with a compression policy
- Redis pub/sub (events) + NATS JetStream (bounded-replay telemetry) fan-out to WebSocket clients
- RBAC (operator/admin) with refresh-token rotation and reuse-chain revocation
- Immutable audit logging, OpenAPI/Swagger docs, OpenTelemetry tracing into Jaeger
- Per-IP rate limiting and a circuit breaker around the Redis/NATS publish paths
- A real end-to-end integration test (gRPC → Postgres → NATS → WebSocket) run in CI
- Dockerfiles and a `docker-compose.prod.yml` for a single-command containerized boot
