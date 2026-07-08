---
title: ADR-001: Axum
layout: default
---

# ADR-001: Axum for the HTTP/WebSocket layer

## Context
The backend needs to serve REST endpoints, upgrade connections to WebSockets for live telemetry
streaming, and share a single async runtime with the gRPC ingestion server and the NATS/Redis
clients (`backend/src/main.rs`).

## Decision
Use Axum on top of Tokio and Tower. Axum's `State<AppState>` extractor gives every handler typed
access to the shared `PgPool`/Redis/NATS clients (`backend/src/main.rs:15-20`), its `ws` feature
covers the telemetry WebSocket endpoint natively (`backend/src/websockets/handler.rs`), and it
composes with `tower` middleware (CORS, the Prometheus `track_metrics` layer, the audit-log layer,
rate limiting) without a separate framework-specific plugin system.

## Consequences
- Axum and Tonic (gRPC) both run on Tokio, so both servers can be spawned from the same `main.rs`
  and share `AppState` by cloning it — no cross-runtime bridging.
- Extractor-based auth (`Claims`/`AdminClaims` implementing `FromRequestParts`) keeps
  authorization declarative in handler signatures instead of imperative checks inside each body.
- Axum 0.7's router is `Send`-heavy by design, which is a good fit for this workload but means
  handler code must stay `Send`-safe throughout (relevant when wiring OpenTelemetry spans through
  async boundaries).
