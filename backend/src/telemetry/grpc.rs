use crate::telemetry::models::CreateTelemetry;
use crate::telemetry::repository::TelemetryRepository;
use crate::AppState;
use tonic::{Request, Response, Status};
use uuid::Uuid;

// Include generated code
pub mod telemetry_proto {
    tonic::include_proto!("telemetry");
}

use telemetry_proto::telemetry_ingest_server::TelemetryIngest;
pub use telemetry_proto::telemetry_ingest_server::TelemetryIngestServer;
use telemetry_proto::{TelemetryRequest, TelemetryResponse};

pub struct MyTelemetryIngest {
    state: AppState,
}

impl MyTelemetryIngest {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl TelemetryIngest for MyTelemetryIngest {
    async fn ingest_telemetry(
        &self,
        request: Request<TelemetryRequest>,
    ) -> Result<Response<TelemetryResponse>, Status> {
        let req = request.into_inner();

        // 1. Validate payload
        let satellite_id = match Uuid::parse_str(&req.satellite_id) {
            Ok(id) => id,
            Err(_) => return Err(Status::invalid_argument("Invalid satellite UUID format")),
        };

        if req.battery_level < 0.0 || req.battery_level > 100.0 {
            return Err(Status::invalid_argument(
                "Invalid battery level (must be 0-100)",
            ));
        }

        let payload = CreateTelemetry {
            satellite_id,
            battery_level: req.battery_level,
            battery_temp: req.battery_temp,
            solar_power: req.solar_power,
            velocity: req.velocity,
            altitude: req.altitude,
            latitude: req.latitude,
            longitude: req.longitude,
        };

        // 2. Upsert Satellite
        let sat_name = match satellite_id.to_string().get(0..8) {
            Some(prefix) => format!("SAT-{}", prefix.to_uppercase()),
            None => "SAT-GENERIC".to_string(),
        };

        let upsert_sat = sqlx::query!(
            r#"
            INSERT INTO satellites (id, name, status)
            VALUES ($1, $2, 'active')
            ON CONFLICT (id) DO NOTHING
            "#,
            satellite_id,
            sat_name
        )
        .execute(&self.state.db)
        .await;

        if let Err(e) = upsert_sat {
            tracing::error!("gRPC: Failed to register satellite on-the-fly: {:?}", e);
            return Err(Status::internal("Database error"));
        }

        // 3. Persist Telemetry Log to PostgreSQL hypertable
        let telemetry = match TelemetryRepository::insert(&self.state.db, &payload).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("gRPC: Failed to insert telemetry record: {:?}", e);
                return Err(Status::internal("Database write error"));
            }
        };

        // Increment telemetry ingested counter
        if let Some(counter) = crate::metrics::TELEMETRY_INGESTED_TOTAL.get() {
            counter.inc();
        }

        // 4. Publish to NATS JetStream for Real-Time Streaming
        if let Ok(serialized) = serde_json::to_string(&telemetry) {
            let subject = format!("telemetry.{}", telemetry.satellite_id);
            let publish_result = self.state.nats.publish(subject, serialized.into()).await;

            if let Err(e) = publish_result {
                tracing::error!(
                    "gRPC: Failed to publish telemetry to NATS JetStream: {:?}",
                    e
                );
            } else {
                tracing::debug!(
                    "gRPC: Telemetry published to NATS JetStream subject 'telemetry.{}'",
                    telemetry.satellite_id
                );
            }
        }

        Ok(Response::new(TelemetryResponse {
            status: "SUCCESS".to_string(),
            message: "Telemetry ingested successfully over gRPC".to_string(),
        }))
    }
}
