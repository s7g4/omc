pub mod checks;

use crate::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

/// Liveness: process is up and able to handle requests. Always 200 if reachable at all.
pub async fn live() -> impl IntoResponse {
    "OK"
}

/// Readiness: dependencies (Postgres, Redis, NATS) are actually reachable.
/// Used to gate whether traffic should be routed to this instance.
pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let status = checks::check_dependencies(&state).await;

    let code = if status.all_healthy() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (code, Json(status))
}
