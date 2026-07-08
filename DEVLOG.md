# Engineering Devlog - Open Mission Control

## Milestone 1: Project Setup & Workspace Initialization
* **Deliverable**: Initialize Cargo Workspace, create base Axum API skeleton, and configure Docker Compose database services.
* **Problems Faced**:
  * **Socket Bind Failure (WSAEACCES / Error 10013)**: The Axum server failed to bind to `127.0.0.1:8080`.
  * **Diagnostic**: Port `8080` was already occupied by active background processes on the Windows host machine.
  * **Resolution**: Shifted the backend HTTP port to `8081` to bypass the collision.

## Milestone 2: Database Schema & Migration Setup
* **Deliverable**: Create Postgres migrations, integrate SQLx connection pool (`PgPool`), and implement the telemetry repository.
* **Problems Faced**:
  * **SQLx Compilation Loop**: Running `cargo check` failed because the `telemetry` table did not exist, yet the migration code was designed to run only when the binary executed.
  * **Resolution**: Manually seeded the database container by piping the migration file into `psql` using `Get-Content backend/migrations/0001_init.sql | docker exec -i omc-postgres psql -U postgres -d omc`.
  * **Port Collisions on Docker Boot**: Native Windows Postgres (`5432`) and Redis (`6379`) services hijacked ports on PC reboot.
  * **Resolution**: Reconfigured `docker-compose.yml` to map host ports `5433` (Postgres) and `6380` (Redis) to the standard container ports, keeping our dev environment completely isolated.
  * **SQLx Migration Duplicate Error**: After manual seeding, the runtime migration threw `relation "users" already exists`.
  * **Resolution**: Reset the database storage by running `docker compose down -v` followed by `docker compose up -d`, allowing SQLx to execute the migrations natively on startup.

## Milestone 3: Satellite Simulator Service
* **Deliverable**: Build the orbital physics engine and simulator configuration.
* **Best Practices**: Added `simulator_config.json` to `.gitignore` to prevent developer-specific UUIDs and local URLs from bleeding into version control.

## Milestone 4: Telemetry Ingestion API
* **Deliverable**: Create `POST /api/v1/telemetry`, implement payload validation, and client-side POST sending.
* **Problems Faced**:
  * **CI Build Failure on GitHub**: The GitHub Actions runner failed to compile `query_as!` macros due to a missing database connection.
  * **Resolution**: Configured SQLx **Offline Mode**. Installed `sqlx-cli` locally, ran `cargo sqlx prepare` to generate cached queries in the `.sqlx/` directory, and added `SQLX_OFFLINE: true` to the `.github/workflows/ci.yml` environment.

## Milestone 5: WebSockets & Real-Time Broadcast
* **Deliverable**: Integrate Redis Pub/Sub, Axum WebSockets (`/api/v1/telemetry/ws`), and event stream loops.
* **Problems Faced**:
  * **Browser CSP Block**: Testing WebSockets from the developer tools console on a secure page (like Bing/MSN) threw a Content Security Policy `connect-src` violation.
  * **Resolution**: Ran the client socket test on `http://example.com`, which has no strict CSP policy.
  * **Embedded Target Compilation Mismatch**: The Rust compiler threw `can't find crate for std` targeting RISC-V because of a global target override.
  * **Resolution**: Created a local `.cargo/config.toml` file to force compiling for `x86_64-pc-windows-msvc`.

## Milestone 6: gRPC Ingestion, TimescaleDB Hypertables, NATS JetStream, Prometheus/Grafana
* **Deliverable**: Dual gRPC/HTTP telemetry ingestion, TimescaleDB hypertable + compression policy, NATS JetStream replay fan-out alongside Redis Pub/Sub, Prometheus metrics + Grafana dashboards.
* See [docs/CASE_STUDY.md](docs/CASE_STUDY.md) and [docs/adr/](docs/adr/) for the reasoning behind these choices in detail.

## Milestone 7: Production Hardening — Auth, Observability, Resilience, Physics, Testing
* **Deliverable**: RBAC (operator/admin roles) with refresh-token rotation and reuse-chain revocation, immutable audit logging, OpenAPI/Swagger docs, layered TOML+env configuration, `/live`/`/ready` health probes, per-IP rate limiting, a hand-rolled circuit breaker around the Redis/NATS publish paths, a physically-grounded rewrite of the simulator (vis-viva orbital velocity, radiative thermal balance, cosine solar law, simulated packet loss/GPS drift), OpenTelemetry tracing into a local Jaeger instance, a k6 load test, a real end-to-end integration test (`backend/tests/e2e_pipeline.rs`), and Dockerfiles/`docker-compose.prod.yml` for a single-command containerized boot.
* **Problems Faced**:
  * **Toolchain corruption**: `rustc` was missing from the active rustup toolchain entirely (reinstalling just the `rustc` component left it version-mismatched against cached `std`/`test` rlibs, producing `found crate 'std' compiled by an incompatible version of rustc`).
  * **Resolution**: `rustup toolchain uninstall stable && rustup toolchain install stable` for a fully consistent reinstall.
  * **`tracing::Span` guard held across `.await`**: Wrapping telemetry ingestion in `tracing::info_span!(...).entered()` and holding the guard across `.await` points made the handler's future non-`Send`, which axum's `Handler` trait requires — surfaced as an opaque "future cannot be sent between threads safely" error pointing at an unrelated line.
  * **Resolution**: Wrapped the handler body in `async move { ... }.instrument(span).await` instead of holding an `EnteredSpan` guard across await points.
  * **Unbounded NATS JetStream replay**: The WebSocket handler's consumer uses `DeliverPolicy::All` so a client connecting mid-mission still sees recent telemetry — but with no retention policy on the stream, it had been accumulating messages across every dev/test session for days, so every *new* dashboard connection replayed the entire backlog before reaching live data (visible as dozens of duplicate stale event-log entries flooding in on connect).
  * **Resolution**: Added a `max_age` retention policy to the JetStream stream config, applied via `update_stream` (not just `get_or_create_stream`) so it also corrects a stream that already existed before the policy was introduced.
  * **`utoipa-swagger-ui` Docker build failure**: Its build script shells out to the system `curl` binary to download Swagger UI's static assets at compile time — failed with an opaque panic (`Os { code: 2, kind: NotFound }`) on the `rust:1-slim-bookworm` builder image, which doesn't include `curl`.
  * **Resolution**: Added `curl` and `ca-certificates` to the builder stage's `apt-get install`.
