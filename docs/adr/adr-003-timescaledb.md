---
title: ADR-003: TimescaleDB
layout: default
---

# ADR-003: TimescaleDB hypertable for telemetry

## Context
Telemetry is written roughly once per second per satellite and queried both as raw recent rows
and as time-bucketed aggregates for charting (`backend/src/telemetry/repository.rs:73-104`). A
plain Postgres table would need manual partitioning to keep query performance from degrading as
history accumulates.

## Decision
Store `telemetry` as a TimescaleDB hypertable partitioned on `created_at`
(`backend/migrations/0003_timescaledb.sql`), with a compression policy segmented by
`satellite_id` that compresses chunks older than 2 hours (`0004_compression.sql`). The dashboard's
history endpoint uses Timescale's `time_bucket()` function directly in SQL
(`repository.rs:82-93`) rather than aggregating in application code.

## Consequences
- Writes stay fast regardless of table size — new data always lands in the current (uncompressed,
  small) chunk.
- `time_bucket()` pushes aggregation down to the database, so the frontend's chart endpoint
  (`GET /api/v1/telemetry/:id/history`) stays a single query instead of fetching raw rows and
  bucketing client-side.
- Compression is segmented by `satellite_id` specifically because queries filter by satellite
  first (`WHERE satellite_id = $1`, matching the index in `0001_init.sql:54`) — segmenting any
  other way would make compressed-chunk queries slower, not faster.
- Requires the `timescale/timescaledb` Postgres image specifically (not plain Postgres) — a
  deliberate constraint accepted because the hypertable/compression features are the reason
  Postgres was chosen over a simpler embedded store.
