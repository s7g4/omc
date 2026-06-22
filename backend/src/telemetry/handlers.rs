use crate::telemetry::models::CreateTelemetry;
use crate::telemetry::repository::TelemetryRepository;
use crate::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use redis::AsyncCommands;

pub async fn ingest_telemetry(
    State(state): State<AppState>,
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
    .execute(&state.db)
    .await;

    if let Err(e) = upsert_sat {
        tracing::error!("Failed to register satellite on-the-fly: {:?}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Database write error").into_response();
    }

    // 3. Persist Telemetry Log to PostgreSQL
    let telemetry = match TelemetryRepository::insert(&state.db, &payload).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to insert telemetry record: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database write error").into_response();
        }
    };

    // 4. Publish to Redis Pub/Sub for Real-Time Streaming
    let mut redis_conn = match state.redis.get_multiplexed_tokio_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Failed to get multiplexed Redis connection: {:?}", e);
            // We still return 201 Created because the data is safely written to Postgres
            return (StatusCode::CREATED, Json(telemetry)).into_response();
        }
    };

    if let Ok(serialized) = serde_json::to_string(&telemetry) {
        let publish_result: Result<(), redis::RedisError> =
            redis_conn.publish("telemetry", serialized).await;

        if let Err(e) = publish_result {
            tracing::error!("Failed to publish telemetry to Redis: {:?}", e);
        } else {
            tracing::debug!("Telemetry published to Redis channel 'telemetry'");
        }
    }

    (StatusCode::CREATED, Json(telemetry)).into_response()
}
