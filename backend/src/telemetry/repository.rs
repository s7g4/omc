#![allow(dead_code)]
use crate::telemetry::models::{CreateTelemetry, Telemetry, TelemetryAggregate};
use sqlx::PgPool;
use uuid::Uuid;

pub struct TelemetryRepository;

impl TelemetryRepository {
    pub async fn insert(
        pool: &PgPool,
        telemetry: &CreateTelemetry,
    ) -> Result<Telemetry, sqlx::Error> {
        sqlx::query_as!(
            Telemetry,
            r#"
            INSERT INTO telemetry (satellite_id, battery_level, battery_temp, solar_power, velocity, altitude, latitude, longitude)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, satellite_id, battery_level, battery_temp, solar_power, velocity, altitude, latitude, longitude, created_at
            "#,
            telemetry.satellite_id,
            telemetry.battery_level,
            telemetry.battery_temp,
            telemetry.solar_power,
            telemetry.velocity,
            telemetry.altitude,
            telemetry.latitude,
            telemetry.longitude
        )
        .fetch_one(pool)
        .await
    }

    pub async fn get_latest(
        pool: &PgPool,
        satellite_id: Uuid,
    ) -> Result<Option<Telemetry>, sqlx::Error> {
        sqlx::query_as!(
            Telemetry,
            r#"
            SELECT id, satellite_id, battery_level, battery_temp, solar_power, velocity, altitude, latitude, longitude, created_at
            FROM telemetry
            WHERE satellite_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            satellite_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn get_history(
        pool: &PgPool,
        satellite_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Telemetry>, sqlx::Error> {
        sqlx::query_as!(
            Telemetry,
            r#"
            SELECT id, satellite_id, battery_level, battery_temp, solar_power, velocity, altitude, latitude, longitude, created_at
            FROM telemetry
            WHERE satellite_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
            satellite_id,
            limit
        )
        .fetch_all(pool)
        .await
    }

    pub async fn get_history_aggregated(
        pool: &PgPool,
        satellite_id: Uuid,
        bucket_seconds: i32,
        limit: i64,
    ) -> Result<Vec<TelemetryAggregate>, sqlx::Error> {
        sqlx::query_as!(
            TelemetryAggregate,
            r#"
            SELECT 
                time_bucket($1 * INTERVAL '1 second', created_at) AS "bucket_time!",
                AVG(battery_level) AS avg_battery_level,
                AVG(battery_temp) AS avg_battery_temp,
                AVG(solar_power) AS avg_solar_power,
                AVG(velocity) AS avg_velocity,
                AVG(altitude) AS avg_altitude,
                AVG(latitude) AS avg_latitude,
                AVG(longitude) AS avg_longitude
            FROM telemetry
            WHERE satellite_id = $2
            GROUP BY time_bucket($1 * INTERVAL '1 second', created_at)
            ORDER BY time_bucket($1 * INTERVAL '1 second', created_at) DESC
            LIMIT $3
            "#,
            f64::from(bucket_seconds),
            satellite_id,
            limit
        )
        .fetch_all(pool)
        .await
    }
}
