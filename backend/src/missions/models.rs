use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow, ToSchema)]
pub struct Mission {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub start_date: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMissionRequest {
    pub name: String,
    pub status: String,
    pub description: Option<String>,
    pub satellite_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMissionRequest {
    pub name: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignSatelliteRequest {
    pub satellite_id: Uuid,
}
