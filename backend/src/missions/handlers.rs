use super::models::{AssignSatelliteRequest, CreateMissionRequest, Mission, UpdateMissionRequest};
use crate::auth::models::Claims;
use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

pub async fn create_mission(
    _claims: Claims, // Require authentication
    State(state): State<AppState>,
    Json(payload): Json<CreateMissionRequest>,
) -> Result<(StatusCode, Json<Mission>), (StatusCode, &'static str)> {
    if payload.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Mission name cannot be empty"));
    }

    let new_id = Uuid::new_v4();
    let mission = sqlx::query_as!(
        Mission,
        r#"
        INSERT INTO missions (id, name, status, description)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, status, description, created_at, start_date
        "#,
        new_id,
        payload.name,
        payload.status,
        payload.description
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create mission",
        )
    })?;

    // If a satellite was specified, assign it to this mission
    if let Some(sat_id) = payload.satellite_id {
        sqlx::query!(
            "UPDATE satellites SET mission_id = $1 WHERE id = $2",
            mission.id,
            sat_id
        )
        .execute(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Mission created but satellite assignment failed",
            )
        })?;
    }

    Ok((StatusCode::CREATED, Json(mission)))
}

pub async fn list_missions(
    _claims: Claims, // Require authentication
    State(state): State<AppState>,
) -> Result<Json<Vec<Mission>>, (StatusCode, &'static str)> {
    let list = sqlx::query_as!(
        Mission,
        "SELECT id, name, status, description, created_at, start_date FROM missions ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch missions list"))?;

    Ok(Json(list))
}

pub async fn get_mission(
    _claims: Claims, // Require authentication
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Mission>, (StatusCode, &'static str)> {
    let mission = sqlx::query_as!(
        Mission,
        "SELECT id, name, status, description, created_at, start_date FROM missions WHERE id = $1",
        id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database query failure"))?;

    match mission {
        Some(m) => Ok(Json(m)),
        None => Err((StatusCode::NOT_FOUND, "Mission not found")),
    }
}

pub async fn update_mission(
    _claims: Claims, // Require authentication
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateMissionRequest>,
) -> Result<Json<Mission>, (StatusCode, &'static str)> {
    let existing = sqlx::query_as!(
        Mission,
        "SELECT id, name, status, description, created_at, start_date FROM missions WHERE id = $1",
        id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database query failure"))?;

    let existing = match existing {
        Some(m) => m,
        None => return Err((StatusCode::NOT_FOUND, "Mission not found")),
    };

    let name = payload.name.unwrap_or(existing.name);
    let status = payload.status.unwrap_or(existing.status);
    let description = payload.description.or(existing.description);

    let updated = sqlx::query_as!(
        Mission,
        r#"
        UPDATE missions
        SET name = $1, status = $2, description = $3
        WHERE id = $4
        RETURNING id, name, status, description, created_at, start_date
        "#,
        name,
        status,
        description,
        id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to update mission",
        )
    })?;

    Ok(Json(updated))
}

pub async fn delete_mission(
    _claims: Claims, // Require authentication
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let result = sqlx::query!("DELETE FROM missions WHERE id = $1", id)
        .execute(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete mission",
            )
        })?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Mission not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn assign_satellite(
    _claims: Claims, // Require authentication
    Path(mission_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<AssignSatelliteRequest>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    // Verify mission exists
    let mission_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM missions WHERE id = $1)",
        mission_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database check failure"))?;

    if !mission_exists.unwrap_or(false) {
        return Err((StatusCode::NOT_FOUND, "Mission not found"));
    }

    // Assign satellite
    let result = sqlx::query!(
        "UPDATE satellites SET mission_id = $1 WHERE id = $2",
        mission_id,
        payload.satellite_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to assign satellite to mission",
        )
    })?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Satellite not found"));
    }

    Ok(StatusCode::OK)
}

pub async fn unassign_satellite(
    _claims: Claims, // Require authentication
    Path(mission_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<AssignSatelliteRequest>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    // Unassign satellite if it is currently assigned to this mission
    let result = sqlx::query!(
        "UPDATE satellites SET mission_id = NULL WHERE id = $1 AND mission_id = $2",
        payload.satellite_id,
        mission_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to unassign satellite",
        )
    })?;

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            "Satellite is not assigned to this mission",
        ));
    }

    Ok(StatusCode::OK)
}
