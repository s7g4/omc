---
title: ADR-004: gRPC
layout: default
---

# ADR-004: gRPC for telemetry ingestion (dual transport with HTTP)

## Context
The simulator sends telemetry once per second, forever, for the lifetime of the process. This is
a different traffic shape than the dashboard's occasional REST calls (login, mission CRUD): high
frequency, machine-to-machine, fixed schema, latency-sensitive.

## Decision
Ingest telemetry over gRPC (`proto/telemetry.proto`, `backend/src/telemetry/grpc.rs`) on a
dedicated port (`GRPC_HOST`/`GRPC_PORT`, default `[::1]:50051`), spawned alongside the Axum HTTP
server in the same process (`backend/src/main.rs:121-140`). The equivalent HTTP endpoint
(`POST /api/v1/telemetry`, `backend/src/telemetry/handlers.rs`) is kept as a second ingestion path
for tooling that doesn't want to link a gRPC client (curl, k6, `tests/load/k6_soak.js`).

## Consequences
- Protobuf's binary encoding and HTTP/2 multiplexing (via `tonic`) suit a client sending the same
  small, fixed-shape message every second better than repeated HTTP/1.1 JSON requests would.
- Both ingestion paths converge on the same logic (upsert satellite, insert telemetry row,
  publish to NATS) — duplicated in `grpc.rs` and `handlers.rs` rather than unified behind one
  entrypoint, which is the one piece of this ADR that's a known simplification: acceptable at this
  scale, would be worth extracting into a shared `TelemetryRepository`-level function if a third
  ingestion path were ever added.
- Running two servers (Axum + Tonic) in one process means both must be independently supervised —
  a panic in the gRPC server's spawned task doesn't currently take down the HTTP server, which is
  fine for a demo but would need a supervisor/restart strategy in a real deployment.
