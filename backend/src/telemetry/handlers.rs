use crate::telemetry::models::CreateTelemetry;
use crate::telemetry::repository::TelemetryRepository;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use sqlx::PgPool;

pub async fn ingest_telemetry(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateTelemetry>,
) -> impl IntoResponse {
    // 1. Data Validation
    if payload.battery_level < 0.0 || payload.battery_level > 100.0 {
        return (
            StatusCode::BAD_REQUEST,
            "Invalid battery level (must be 0-100)",
        )
            .into_response();
    }
    if payload.latitude < -90.0 || payload.latitude > 90.0 {
        return (
            StatusCode::BAD_REQUEST,
            "Invalid latitude (must be -90 to 90)",
        )
            .into_response();
    }
    if payload.longitude < -180.0 || payload.longitude > 180.0 {
        return (
            StatusCode::BAD_REQUEST,
            "Invalid longitude (must be -180 to 180)",
        )
            .into_response();
    }

    // 2. Ensure Satellite Exists (Upsert pattern)
    // Generates a display name from the first 8 characters of the UUID (e.g. SAT-3a9f8b4d)
    let sat_name = format!("SAT-{}", &payload.satellite_id.to_string()[..8]);

    let upsert_sat = sqlx::query!(
        r#"
        INSERT INTO satellites (id, name, status)
        VALUES ($1, $2, 'active')
        ON CONFLICT (id) DO NOTHING
        "#,
        payload.satellite_id,
        sat_name
    )
    .execute(&pool)
    .await;

    if let Err(e) = upsert_sat {
        tracing::error!("Failed to register satellite on-the-fly: {:?}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Database write error").into_response();
    }

    // 3. Persist Telemetry Log
    match TelemetryRepository::insert(&pool, &payload).await {
        Ok(telemetry) => {
            tracing::info!(
                "Successfully ingested telemetry for satellite: {}",
                telemetry.satellite_id
            );
            (StatusCode::CREATED, Json(telemetry)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to insert telemetry record: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database write error").into_response()
        }
    }
}
