pub mod models;
pub mod repository;

use crate::auth::middleware::AdminClaims;
use crate::AppState;
use axum::{
    body::Body,
    extract::{ConnectInfo, MatchedPath, Query, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use models::NewAuditLog;
use std::net::SocketAddr;

#[derive(serde::Deserialize)]
pub struct ListAuditLogsParams {
    pub limit: Option<i64>,
}

/// Admin-only: browse the immutable audit trail. Demonstrates that `audit_log_layer` below is
/// actually populating `audit_logs`, not just writing to a table nothing ever reads.
pub async fn list_audit_logs(
    _claims: AdminClaims,
    State(state): State<AppState>,
    Query(params): Query<ListAuditLogsParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(500);
    match repository::list_recent(&state.db, limit).await {
        Ok(logs) => (StatusCode::OK, Json(logs)).into_response(),
        Err(e) => {
            tracing::error!("Failed to list audit logs: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database query failure").into_response()
        }
    }
}

/// Best-effort decode of the bearer token for actor attribution. Never fails the request —
/// audit logging is observability, not an auth gate (that's `Claims`/`AdminClaims`).
fn decode_actor(req: &Request<Body>) -> (Option<uuid::Uuid>, Option<String>) {
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "mission_control_default_secret_key_12345".to_string());

    let claims = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(|token| {
            decode::<crate::auth::models::Claims>(
                token,
                &DecodingKey::from_secret(secret.as_bytes()),
                &Validation::default(),
            )
            .ok()
        });

    match claims {
        Some(data) => (data.claims.sub.parse().ok(), Some(data.claims.username)),
        None => (None, None),
    }
}

pub async fn audit_log_layer(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().to_string();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());
    let source_ip = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string());
    let (actor_id, actor_username) = decode_actor(&req);

    let response = next.run(req).await;
    let status_code = response.status().as_u16() as i32;

    let db = state.db.clone();
    tokio::spawn(async move {
        let entry = NewAuditLog {
            actor_id,
            actor_username,
            method,
            path,
            status_code,
            source_ip,
        };
        if let Err(e) = repository::insert(&db, entry).await {
            tracing::warn!("Failed to write audit log entry: {:?}", e);
        }
    });

    response
}
