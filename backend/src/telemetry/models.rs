#![allow(dead_code)]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
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
