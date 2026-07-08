//! Black-box integration test: proves the full pipeline gRPC -> Postgres -> NATS JetStream ->
//! WebSocket actually works together, not just in isolation. Runs the real compiled `backend`
//! binary as a subprocess (via `CARGO_BIN_EXE_backend`) against the docker-compose services, so
//! it exercises exactly what ships, not a mocked-out version of it.
//!
//! Requires `docker compose up -d postgres redis nats` running first (see CI's
//! `integration-tests` job, or run it locally the same way).

use futures_util::StreamExt;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

pub mod telemetry_proto {
    tonic::include_proto!("telemetry");
}
use telemetry_proto::telemetry_ingest_client::TelemetryIngestClient;
use telemetry_proto::TelemetryRequest;

const HTTP_PORT: &str = "18081";
const GRPC_PORT: &str = "15051";

/// Kills the backend subprocess on drop so a failing assertion (which unwinds past the
/// `Child` before we get to call `.kill()` manually) doesn't leak an orphaned server.
struct BackendProcess(Child);

impl Drop for BackendProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_backend() -> BackendProcess {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password123@127.0.0.1:5433/omc".to_string());
    let redis_url =
        std::env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".to_string());
    let nats_url =
        std::env::var("TEST_NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());

    let child = Command::new(env!("CARGO_BIN_EXE_backend"))
        .env("DATABASE_URL", database_url)
        .env("REDIS_URL", redis_url)
        .env("NATS_URL", nats_url)
        .env("JWT_SECRET", "e2e-test-secret")
        .env("HTTP_HOST", "127.0.0.1")
        .env("HTTP_PORT", HTTP_PORT)
        .env("GRPC_HOST", "127.0.0.1")
        .env("GRPC_PORT", GRPC_PORT)
        .env("APP__OTEL__ENABLED", "false")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn backend binary");

    BackendProcess(child)
}

async fn wait_for_health() {
    let client = reqwest::Client::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);

    loop {
        if tokio::time::Instant::now() > deadline {
            panic!("backend did not become healthy within 30s");
        }
        if let Ok(resp) = client
            .get(format!("http://127.0.0.1:{HTTP_PORT}/health"))
            .send()
            .await
        {
            if resp.status().is_success() {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[tokio::test]
async fn telemetry_flows_from_grpc_through_db_and_nats_to_websocket() {
    let _backend = spawn_backend();
    wait_for_health().await;

    let satellite_id = Uuid::new_v4();

    // 1. Open the WebSocket first (NATS JetStream's DeliverPolicy::All means order doesn't
    // actually matter for delivery, but connecting first is the more realistic ordering and
    // keeps this test honest about what a real dashboard session does).
    let ws_url =
        format!("ws://127.0.0.1:{HTTP_PORT}/api/v1/telemetry/ws?satellite_id={satellite_id}");
    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("failed to open telemetry websocket");
    let (_ws_write, mut ws_read) = ws_stream.split();

    // 2. Send one telemetry reading over gRPC — the same path the simulator uses.
    let mut grpc_client = TelemetryIngestClient::connect(format!("http://127.0.0.1:{GRPC_PORT}"))
        .await
        .expect("failed to connect gRPC client");

    let response = grpc_client
        .ingest_telemetry(tonic::Request::new(TelemetryRequest {
            satellite_id: satellite_id.to_string(),
            battery_level: 87.5,
            battery_temp: 22.0,
            solar_power: 140.0,
            velocity: 7.66,
            altitude: 500.0,
            latitude: 12.34,
            longitude: -56.78,
        }))
        .await
        .expect("gRPC ingest_telemetry call failed")
        .into_inner();

    assert_eq!(response.status, "SUCCESS");

    // 3. Assert the row actually landed in the TimescaleDB hypertable.
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password123@127.0.0.1:5433/omc".to_string());
    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("failed to connect to postgres for assertions");

    let row: (f64,) = sqlx::query_as(
        "SELECT battery_level FROM telemetry WHERE satellite_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(satellite_id)
    .fetch_one(&pool)
    .await
    .expect("telemetry row was not persisted to Postgres");
    assert!((row.0 - 87.5).abs() < f64::EPSILON);

    // 4. Assert the same reading was fanned out over NATS JetStream to the WebSocket client —
    // proving the gRPC handler's publish and the websocket handler's consumer are actually
    // wired together, not just independently functional.
    let received = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match ws_read.next().await {
                Some(Ok(Message::Text(text))) => {
                    let payload: serde_json::Value =
                        serde_json::from_str(&text).expect("non-JSON websocket message");
                    if payload.get("satellite_id").and_then(|v| v.as_str())
                        == Some(satellite_id.to_string().as_str())
                    {
                        return payload;
                    }
                    // Not our message (e.g. an event broadcast); keep waiting.
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => panic!("websocket error while waiting for telemetry: {e:?}"),
                None => panic!("websocket closed before telemetry arrived"),
            }
        }
    })
    .await
    .expect("timed out waiting for telemetry over the websocket");

    assert_eq!(
        received.get("battery_level").and_then(|v| v.as_f64()),
        Some(87.5)
    );
}
