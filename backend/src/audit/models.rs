use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct AuditLog {
    pub id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub actor_id: Option<Uuid>,
    pub actor_username: Option<String>,
    pub method: String,
    pub path: String,
    pub status_code: i32,
    pub source_ip: Option<String>,
}

pub struct NewAuditLog {
    pub actor_id: Option<Uuid>,
    pub actor_username: Option<String>,
    pub method: String,
    pub path: String,
    pub status_code: i32,
    pub source_ip: Option<String>,
}
