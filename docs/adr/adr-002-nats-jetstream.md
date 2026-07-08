---
title: ADR-002: NATS JetStream
layout: default
---

# ADR-002: NATS JetStream alongside Redis Pub/Sub

## Context
Two different kinds of real-time data flow out of the backend: high-frequency telemetry (needs to
be replayable — a dashboard that connects mid-mission should still see recent history) and
one-off operator events (fault injections, mission changes — pure fire-and-forget notifications).

## Decision
Use NATS JetStream for telemetry and Redis Pub/Sub for events, both fanned into the same
WebSocket connection (`backend/src/websockets/handler.rs:56-121`).

- **NATS JetStream**: telemetry is published to `telemetry.<satellite_id>` and persisted in the
  `TELEMETRY_STREAM` (`backend/src/main.rs:53-62`). The WebSocket handler opens a pull consumer
  with `DeliverPolicy::All` (`handler.rs:72-78`), so a client connecting after telemetry was
  already sent still receives it — a plain pub/sub system would silently drop anything published
  before the subscriber connected.
- **Redis Pub/Sub**: operator events (`inject_fault`, `backend/src/telemetry/handlers.rs`) don't
  need replay — they're transient notifications, and Redis pub/sub's simplicity and existing use
  as the simulator's control channel (`simulator:control`) made it the lower-friction choice for
  this half of the traffic.

## Consequences
- Two message systems means two failure modes to handle independently — this is why both publish
  paths are wrapped in a circuit breaker (`backend/src/resilience.rs`) rather than one shared one.
- JetStream's replay means the stream grows unbounded unless retention policy is set; acceptable
  for a demo, would need a retention/`max_age` policy in a longer-lived deployment.
- A single WebSocket connection has to `tokio::select!` over both sources
  (`handler.rs:99-134`), which is straightforward with Tokio but does mean the two systems'
  message formats (`telemetry.<id>` payload vs. `events` payload) both need to be plain JSON
  strings the frontend can distinguish by shape (`severity`/`message` fields present or absent).
