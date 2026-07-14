---
title: Case Study
layout: default
---

# Case Study: Engineering Decisions Behind Open Mission Control

A narrative walkthrough of the non-obvious decisions in this codebase and why they were made.
For terser, one-decision-per-file versions of the same material, see [docs/adr/](adr/).

## Why a satellite telemetry platform, and why this stack

The goal was a project that forces real distributed-systems problems to show up, without needing
actual satellite hardware: high-frequency time-series writes, a real-time fan-out problem (one
telemetry stream, many possible dashboard viewers), and a "commands go one way, telemetry comes
back the other way" control loop. A synthetic orbital-physics simulator generating gRPC traffic
gave all three without any external dependency on real hardware or a paid data feed.

## Why TimescaleDB instead of plain Postgres

Telemetry writes are frequent (once a second per satellite) and read patterns are two different
shapes: "give me the last N raw rows" and "give me time-bucketed averages for a chart." A plain
Postgres table would need manual partitioning to keep either query pattern fast as data
accumulates. TimescaleDB's hypertable does that automatically, and its `time_bucket()` SQL
function pushes the chart aggregation into the database instead of pulling raw rows and
aggregating in the API layer. The compression policy (segment by `satellite_id`, compress after 2
hours) means old data shrinks instead of growing the table forever, without needing a separate
archival job. See [ADR-003](adr/adr-003-timescaledb.md) for the detail.

## Why gRPC *and* HTTP for telemetry ingestion, not just one

The simulator is a long-running process sending the same small, fixed-shape message once a
second, forever — that's a good fit for gRPC's binary encoding and persistent HTTP/2 connection.
But keeping the HTTP endpoint too meant `curl`, k6, and this project's own test suite never needed
a gRPC client to exercise the ingestion path. The cost is that the upsert-satellite /
insert-telemetry / publish-to-NATS logic exists twice (`telemetry/grpc.rs` and
`telemetry/handlers.rs`) — a known, accepted duplication rather than an oversight; see
[ADR-004](adr/adr-004-grpc.md).

## Why NATS JetStream *and* Redis Pub/Sub, not just one

These solve different problems. Telemetry needs replay: a dashboard connecting mid-mission should
still see recent history, not just whatever arrives after it happens to connect. NATS JetStream's
`DeliverPolicy::All` consumer gives that for free. Operator events (fault injections) don't need
replay — they're one-off notifications, and Redis was already in use as the simulator's control
channel, so reusing it for events avoided a third message system. Both publish paths ended up
wrapped in the same lightweight circuit breaker (`backend/src/resilience.rs`) once it became clear
they fail independently and both currently just log-and-continue on failure without it. Full
reasoning in [ADR-002](adr/adr-002-nats-jetstream.md).

**The one bug this combination actually produced**: without a retention policy, the JetStream
stream accumulates every message ever published to it, forever. Because `DeliverPolicy::All`
replays the *entire* stream to a newly-connecting consumer, every fresh dashboard session was
replaying days of old test telemetry before catching up to live data — visible as a flood of
duplicate/stale event-log entries on connect. Fixed with a `max_age` retention policy on the
stream (bounding the replay window to 10 minutes) applied via `update_stream` rather than just
`get_or_create_stream`, so it also corrects a stream that already existed before the policy was
added. This is the kind of thing that's invisible in a quick local test and only shows up once a
stream has accumulated real history — exactly what the integration test suite and a longer-running
local session caught.

## Why RBAC is two roles, not a permissions matrix

`operator` and `admin` cover exactly the actions this platform actually has: everyone can view
telemetry and manage their own missions, only admins can delete missions or unassign satellites
from them. A general permissions/policy engine would be solving a problem this app doesn't have
yet. The `AdminClaims` extractor (`backend/src/auth/middleware.rs`) wraps the existing `Claims`
extractor rather than duplicating JWT validation — the role check is additive, not a parallel auth
path.

## Why refresh tokens are opaque, hashed, random strings — not JWTs

The access token is a JWT (self-contained, verifiable without a DB round-trip, short-lived at 1
hour). The refresh token is deliberately *not* a JWT: it's a random 32-byte value, stored as a
SHA-256 hash (`refresh_tokens.token_hash`), because a refresh token's whole job is to be
revocable, and a self-contained JWT can't be revoked without either a blocklist (which is just a
database table anyway) or waiting out its expiry. Making it an opaque token stored server-side
means revocation is a single `UPDATE ... SET revoked_at = NOW()`, and reuse detection (a stolen
refresh token gets used twice — once by the attacker, once by the legitimate client) is a single
`WHERE revoked_at IS NOT NULL` check that revokes the entire rotation chain.

## Why the circuit breaker is hand-rolled instead of a crate

It's a ~50-line atomic-counter state machine (closed → open after N failures → half-open after a
cooldown). Pulling in an external crate for that trades a small, auditable amount of code for a
dependency whose behavior needs the same amount of reading-the-source to trust. Given the actual
requirement here — stop hammering a NATS/Redis connection that's already failing, not implement a
general-purpose resilience framework — hand-rolling it was the more honest amount of engineering.

## What the integration test actually proves

`backend/tests/e2e_pipeline.rs` doesn't unit-test a function — it spawns the real compiled backend
binary as a subprocess (via `CARGO_BIN_EXE_backend`, so it's testing exactly what ships, not a
mock of it), sends one telemetry reading over gRPC, and asserts two independent things: the row
exists in TimescaleDB, *and* the same reading arrives over a live WebSocket connection. Either
assertion could pass while the other silently breaks (e.g. the DB write succeeds but the NATS
publish is broken, or vice versa) — testing both closes exactly the gap the project's own roadmap
flagged as the main risk: individually-working components that were never proven to work
*together*.

## Migrations are one-way

`sqlx::migrate!` (`backend/src/db.rs`) applies migrations linearly and has no built-in "down"
step. That's fine for this project's lifecycle — schema changes are additive and there's no
production data to protect a rollback path for — but it's worth being explicit that the recovery
story for a bad migration here is "restore from a backup," not "migrate down," since sqlx doesn't
support reversible migrations without hand-writing a parallel set of down-scripts this project
doesn't have.

## A known, deliberately-unfixed `npm audit` finding

`npm audit` reports a moderate PostCSS advisory, reached transitively through Next.js's own
bundled copy (`next/node_modules/postcss`, not the top-level one Tailwind uses). `npm audit fix
--force` "resolves" it by downgrading `next` from 16.2.9 to a `9.3.3-canary` release — a
regression far more severe than the advisory itself, and not something the actual fix (Next.js
bumping its bundled PostCSS upstream) requires. Applying the suggested fix here would be worse
engineering than leaving the advisory open with this explanation attached. CI runs `npm audit
--audit-level=high` as an informational, non-blocking step so this doesn't silently regress if a
*new* advisory shows up that isn't this one.

## What's explicitly out of scope, and why

No Kubernetes manifests, no Terraform, no HashiCorp Vault, no LLM-backed features, and no live
public hosting. This is a demo/portfolio deployment shape (`docker compose up`), not a production
SaaS — introducing orchestration and secrets-management tooling sized for a multi-tenant
production system, or paying for cloud hosting, would be solving problems this deployment doesn't
have. That's a deliberate scope boundary, not an oversight.
