// Local load/soak test against the HTTP telemetry ingestion path
// (POST /api/v1/telemetry) — simpler to drive from k6 than the gRPC path.
//
// Run:
//   k6 run tests/load/k6_soak.js
//   BASE_URL=http://localhost:8081 k6 run tests/load/k6_soak.js
//
// Note on scale: this defaults to a load level that's actually achievable on a laptop
// against a single Postgres/TimescaleDB instance with the default 5-connection pool
// (see `PgPoolOptions::max_connections(5)` in backend/src/db.rs) — tens of VUs sending a
// request roughly once a second each, run for ~90s. That is NOT "100k msgs/sec" scale;
// reaching that would need a tuned connection pool, a multi-instance backend behind a load
// balancer, and dedicated hardware. To scale this script up: raise `vus`/`duration` below,
// and raise `sqlx::PgPoolOptions::max_connections` and Postgres's own `max_connections`
// to match.
import http from "k6/http";
import { check, sleep } from "k6";
import { uuidv4 } from "https://jslib.k6.io/k6-utils/1.4.0/index.js";

const BASE_URL = __ENV.BASE_URL || "http://localhost:8081";

export const options = {
  scenarios: {
    soak: {
      executor: "constant-vus",
      vus: 30,
      duration: "90s",
    },
  },
  thresholds: {
    http_req_failed: ["rate<0.01"],
    http_req_duration: ["p(95)<500"],
  },
};

// A handful of fixed satellite ids so telemetry accumulates into a realistic number of
// hypertable partitions instead of one row per virtual user per satellite.
const SATELLITE_IDS = Array.from({ length: 5 }, () => uuidv4());

export default function () {
  const satelliteId = SATELLITE_IDS[Math.floor(Math.random() * SATELLITE_IDS.length)];

  const payload = JSON.stringify({
    satellite_id: satelliteId,
    battery_level: 40 + Math.random() * 60,
    battery_temp: 10 + Math.random() * 30,
    solar_power: Math.random() * 180,
    velocity: 7.5 + Math.random() * 0.3,
    altitude: 400 + Math.random() * 200,
    latitude: Math.random() * 180 - 90,
    longitude: Math.random() * 360 - 180,
  });

  const res = http.post(`${BASE_URL}/api/v1/telemetry`, payload, {
    headers: { "Content-Type": "application/json" },
  });

  check(res, {
    "status is 201": (r) => r.status === 201,
  });

  sleep(1);
}
