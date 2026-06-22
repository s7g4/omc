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
