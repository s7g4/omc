# Demo Walkthrough

Two ways to run this: the full containerized stack (one command, closest to "production"), or
running each piece manually (better for active development — instant recompiles, real logs).

## Option A — one command, fully containerized

```bash
git clone <this repo> && cd omc
docker compose -f docker-compose.prod.yml up --build
```

First build takes a few minutes (compiling two Rust binaries + the Next.js app from scratch).
Once it settles:

| What | URL |
|---|---|
| Dashboard | http://localhost:3000 |
| Backend health | http://localhost:8081/health → `OK` |
| Backend readiness | http://localhost:8081/ready → `{"postgres":true,"redis":true,"nats":true}` |
| Swagger / OpenAPI UI | http://localhost:8081/swagger-ui |
| Jaeger (traces) | http://localhost:16686 |
| Prometheus | http://localhost:9090 |
| Grafana | http://localhost:3001 (admin/admin) |

The `simulator` container starts streaming synthetic telemetry for a satellite immediately —
open the dashboard, log in (see below), and it's already live.

## Option B — manual, for development

```bash
docker compose up -d                    # Postgres/TimescaleDB, Redis, NATS, Prometheus, Grafana, Jaeger
cd backend && cargo run                 # Axum + gRPC backend on :8081 / :50051
cd simulator && cargo run               # synthetic satellite telemetry over gRPC
cd frontend && npm install && npm run dev   # dashboard on :3000
```

## Logging in

The dashboard has no seeded account — register one from the login screen (or via curl):

```bash
curl -X POST http://localhost:8081/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"operator1","password":"a real password"}'
```

New accounts default to the `operator` role. A few actions (deleting a mission, unassigning a
satellite) require `admin`. Promote one manually:

```bash
docker exec omc-postgres psql -U postgres -d omc \
  -c "UPDATE users SET role='admin' WHERE username='operator1';"
```

Then log out and back in — the JWT's role claim is set at login time, so an already-issued token
keeps the role it had when it was minted.

## What you should see

1. **Telemetry Live tab**: altitude/velocity/battery/temperature updating roughly once a second,
   sourced from the simulator's orbital physics model over gRPC → Postgres → NATS JetStream →
   WebSocket. "UPLINK FEED: ONLINE" in the header confirms the WebSocket is connected.
2. **Simulator Ingress tab**: click a fault (e.g. "Battery Rapid Drain") — it POSTs to
   `/api/v1/simulator/inject`, which logs an event, publishes a control message to the simulator
   over Redis, and broadcasts the event over the same WebSocket. Watch the battery reading drop
   and the event show up in the Terminal Console tab within a second or two. Click "Reset Sim" to
   clear it.
3. **Missions tab**: create a mission; as a non-admin, try deleting one — expect a 403. Promote
   to admin (above) and log back in to confirm delete now works (204).
4. **Terminal Console tab**: the live event stream, filterable by severity.
5. **Timescale DB tab**: the same telemetry history the History endpoint serves, aggregated via
   TimescaleDB's `time_bucket()`.

## Poking the API directly

```bash
# Readiness (kill redis and re-curl to see it flip to 503 with a breakdown):
curl -s http://localhost:8081/ready | python3 -m json.tool

# Refresh token rotation — reusing an old token after rotating is rejected:
curl -s -X POST http://localhost:8081/api/v1/auth/refresh \
  -H "Content-Type: application/json" -d '{"refresh_token":"<token from login response>"}'

# Admin-only audit trail (who did what, when):
curl -s "http://localhost:8081/api/v1/audit-logs?limit=20" \
  -H "Authorization: Bearer <admin access token>" | python3 -m json.tool

# Prometheus metrics:
curl -s http://localhost:8081/metrics | grep telemetry_ingested_total
```

## Tracing a request through Jaeger

Every telemetry ingest (HTTP or gRPC) opens a span tagged with a `trace_id` that's also embedded
in the JSON payload published to NATS — so the same id correlates the trace, the audit log entry,
and the message a WebSocket client receives. Open http://localhost:16686, select service
`omc-backend`, and look for the `telemetry_ingest` operation.

## Load testing

```bash
# Install k6: https://k6.io/docs/get-started/installation/
k6 run tests/load/k6_soak.js
```

Defaults to 30 virtual users against `POST /api/v1/telemetry` for 90 seconds — see the comment
at the top of `tests/load/k6_soak.js` for how to scale it up, and what's actually realistic on a
laptop against a single Postgres instance.

## Running the integration test

```bash
docker compose up -d postgres redis nats
cd backend && cargo test --test e2e_pipeline -- --ignored --nocapture
```

This spins up the real compiled backend binary and asserts telemetry sent over gRPC actually
lands in TimescaleDB *and* gets pushed out over the WebSocket — the two things that are easy to
break independently without anything catching it.
