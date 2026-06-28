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

#[derive(serde::Deserialize)]
pub struct InjectFaultRequest {
    pub satellite_id: uuid::Uuid,
    pub fault: Option<String>,
}

#[derive(serde::Serialize)]
struct RedisControlMsg {
    satellite_id: uuid::Uuid,
    command: &'static str,
    fault: Option<String>,
}

#[derive(serde::Serialize)]
struct WsEventMsg {
    event_id: uuid::Uuid,
    satellite_id: uuid::Uuid,
    severity: &'static str,
    message: String,
    timestamp: String,
}

pub async fn inject_fault(
    State(state): State<AppState>,
    _claims: crate::auth::models::Claims,
    Json(payload): Json<InjectFaultRequest>,
) -> impl IntoResponse {
    let severity = if payload.fault.is_some() {
        "warning"
    } else {
        "info"
    };
    let msg = if let Some(ref f) = payload.fault {
        format!("Manual override fault injected: [{}]", f)
    } else {
        "Manual override fault registers reset nominal".to_string()
    };
    let event_id = uuid::Uuid::new_v4();

    // 1. Log event to Postgres database events table
    let insert_event = sqlx::query!(
        r#"
        INSERT INTO events (id, satellite_id, severity, message)
        VALUES ($1, $2, $3, $4)
        "#,
        event_id,
        payload.satellite_id,
        severity,
        msg
    )
    .execute(&state.db)
    .await;

    if let Err(e) = insert_event {
        tracing::error!("Failed to log fault injection event: {:?}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Database write error").into_response();
    }

    // 2. Dispatch command to Redis Pub/Sub
    let mut redis_conn = match state.redis.get_multiplexed_tokio_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Failed to connect to Redis for command link: {:?}", e);
            return (
                StatusCode::ACCEPTED,
                "Event logged to DB; Redis stream failed",
            )
                .into_response();
        }
    };

    let control_msg = RedisControlMsg {
        satellite_id: payload.satellite_id,
        command: "inject_fault",
        fault: payload.fault.clone(),
    };

    let ws_event = WsEventMsg {
        event_id,
        satellite_id: payload.satellite_id,
        severity,
        message: msg,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    if let Ok(serialized_control) = serde_json::to_string(&control_msg) {
        let _: Result<(), _> = redis_conn
            .publish("simulator:control", serialized_control)
            .await;
    }

    if let Ok(serialized_event) = serde_json::to_string(&ws_event) {
        let _: Result<(), _> = redis_conn.publish("events", serialized_event).await;
    }

    (StatusCode::ACCEPTED, "Override requested").into_response()
}
