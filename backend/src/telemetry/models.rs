#![allow(dead_code)]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow, ToSchema)]
pub struct Telemetry {
    pub id: i64,
    pub satellite_id: Uuid,
    pub battery_level: f64,
    pub battery_temp: f64,
    pub solar_power: f64,
    pub velocity: f64,
    pub altitude: f64,
    pub latitude: f64,
    pub longitude: f64,
    pub created_at: DateTime<Utc>,
}

impl Telemetry {
    /// JSON payload published to NATS, tagged with a per-request trace id so the same id
    /// shows up in the OTEL span, this message, and the audit log entry for the ingest call —
    /// a lightweight stand-in for full W3C trace-context propagation through gRPC/NATS.
    pub fn to_traced_json(&self, trace_id: Uuid) -> Result<String, serde_json::Error> {
        #[derive(Serialize)]
        struct TracedTelemetry<'a> {
            #[serde(flatten)]
            telemetry: &'a Telemetry,
            trace_id: Uuid,
        }
        serde_json::to_string(&TracedTelemetry {
            telemetry: self,
            trace_id,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CreateTelemetry {
    pub satellite_id: Uuid,
    pub battery_level: f64,
    pub battery_temp: f64,
    pub solar_power: f64,
    pub velocity: f64,
    pub altitude: f64,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetryAggregate {
    pub bucket_time: DateTime<Utc>,
    pub avg_battery_level: Option<f64>,
    pub avg_battery_temp: Option<f64>,
    pub avg_solar_power: Option<f64>,
    pub avg_velocity: Option<f64>,
    pub avg_altitude: Option<f64>,
    pub avg_latitude: Option<f64>,
    pub avg_longitude: Option<f64>,
}
